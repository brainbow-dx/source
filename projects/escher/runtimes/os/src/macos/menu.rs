use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::sel;
use objc2::MainThreadMarker;

use objc2_app_kit::NSApplication;
use objc2_app_kit::NSMenu;
use objc2_app_kit::NSMenuItem;

use objc2_foundation::NSString;

use crate::menu::MenuItem;
use crate::menu::MenuRole;
use crate::OsError;

/// (title, selector, key equivalent) for each role — all standard `NSResponder`/`NSApplication`/
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
        MenuItem::Item { label, key_equivalent } => unsafe {
            // No selector — visible but inert until real click-handler wiring exists (see this
            // module's parent doc comment).
            NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &NSString::from_str(label), None, &NSString::from_str(key_equivalent))
        },
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
