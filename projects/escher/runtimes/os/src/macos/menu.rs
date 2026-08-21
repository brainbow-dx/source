use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::runtime::Sel;
use objc2::sel;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};

use objc2_app_kit::NSApplication;
use objc2_app_kit::NSMenu;
use objc2_app_kit::NSMenuItem;

use objc2_foundation::NSObject;
use objc2_foundation::NSObjectProtocol;
use objc2_foundation::NSString;

use crate::menu::MenuItem;
use crate::menu::MenuRole;
use crate::OsError;

/// Bridges a custom `MenuItem::Item`'s `action` into AppKit's target-action mechanism. An
/// `NSMenuItem`'s `action` selector needs a real Objective-C target object to fire on, same as
/// `NSButton`. Duplicated from `escher_appkit::action::ActionTarget`'s shape rather than depended
/// on (this crate can't depend on `escher-appkit`, which already depends on it).
struct MenuActionTargetIvars {
    action: Arc<dyn Fn() + Send + Sync>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `MenuActionTarget` doesn't implement
    // `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = MenuActionTargetIvars]
    struct MenuActionTarget;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for MenuActionTarget {}

    impl MenuActionTarget {
        // SAFETY: matches the `(id)sender` signature every `NSMenuItem` action selector is
        // invoked with. `sender` is never read, so its exact type doesn't matter here.
        #[unsafe(method(fire:))]
        fn fire(&self, _sender: &AnyObject) {
            (self.ivars().action)();
        }
    }
);

impl MenuActionTarget {
    fn new(mtm: MainThreadMarker, action: Arc<dyn Fn() + Send + Sync>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MenuActionTargetIvars { action });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}

/// (title, selector, key equivalent) for each role. All are standard `NSResponder`/`NSApplication`/
/// `NSText` action methods that AppKit routes through the responder chain on its own, which is
/// exactly why no target object is needed here.
fn role_parts(role: MenuRole) -> (&'static str, Sel, &'static str) {
    match role {
        MenuRole::Quit => ("Quit", sel!(terminate:), "q"),
        MenuRole::Hide => ("Hide", sel!(hide:), "h"),
        MenuRole::HideOthers => ("Hide Others", sel!(hideOtherApplications:), ""),
        MenuRole::ShowAll => ("Show All", sel!(unhideAllApplications:), ""),
        MenuRole::CloseWindow => ("Close Window", sel!(performClose:), "w"),
        MenuRole::Undo => ("Undo", sel!(undo:), "z"),
        MenuRole::Redo => ("Redo", sel!(redo:), "Z"),
        MenuRole::Cut => ("Cut", sel!(cut:), "x"),
        MenuRole::Copy => ("Copy", sel!(copy:), "c"),
        MenuRole::Paste => ("Paste", sel!(paste:), "v"),
        MenuRole::SelectAll => ("Select All", sel!(selectAll:), "a"),
    }
}

fn build_item(mtm: MainThreadMarker, item: &MenuItem) -> Retained<NSMenuItem> {
    match item {
        MenuItem::Separator => NSMenuItem::separatorItem(mtm),
        MenuItem::Role(role) => {
            let (title, selector, key_equivalent) = role_parts(*role);
            unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    &NSString::from_str(title),
                    Some(selector),
                    &NSString::from_str(key_equivalent),
                )
            }
        }
        MenuItem::Item { label, key_equivalent, action } => {
            let menu_item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &NSString::from_str(label), Some(sel!(fire:)), &NSString::from_str(key_equivalent))
            };

            let target = MenuActionTarget::new(mtm, action.clone());
            // SAFETY: `target` is a real `MenuActionTarget` responding to `fire:` exactly as set
            // above; it's kept alive for the process's whole lifetime (see the `forget` below).
            unsafe { menu_item.setTarget(Some(&target)) };
            // `NSMenuItem.target` is a weak property. Nothing else keeps `target` retained past
            // this call. The application menu bar is installed exactly once, at startup, and lives
            // for the process's whole lifetime with no teardown path to release this into, so
            // leaking this one retain permanently is deliberate, not an oversight.
            std::mem::forget(target);

            menu_item
        }
        MenuItem::Submenu { label, items } => {
            let menu_item = unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &NSString::from_str(label), None, &NSString::from_str("")) };
            let submenu = build_menu(mtm, label, items);
            menu_item.setSubmenu(Some(&submenu));
            menu_item
        }
    }
}

fn build_menu(mtm: MainThreadMarker, title: &str, items: &[MenuItem]) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    menu.setTitle(&NSString::from_str(title));

    for item in items {
        menu.addItem(&build_item(mtm, item));
    }

    menu
}

pub fn set_application_menu(items: &[MenuItem]) -> Result<(), OsError> {
    let mtm = MainThreadMarker::new().ok_or(OsError::NotOnMainThread)?;

    let main_menu = build_menu(mtm, "", items);
    NSApplication::sharedApplication(mtm).setMainMenu(Some(&main_menu));

    Ok(())
}
