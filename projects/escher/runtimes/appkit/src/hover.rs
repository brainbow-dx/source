//! Pointing-hand cursor + hover-state notification for any native view. `NSTrackingArea`'s
//! `owner` must be a real Objective-C object responding to `mouseEntered:`/`mouseExited:` — this
//! is that object, mirroring why `crate::action::ActionTarget` exists (bridging an AppKit
//! callback mechanism into a plain Rust closure). Works on stock `NSButton`/`NSView` instances
//! without subclassing them — the tracking area (not the view itself) is what needs a custom
//! owner.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSCursor, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{NSObject, NSObjectProtocol, NSRect};

pub struct HoverTargetIvars {
    on_change: RefCell<Box<dyn FnMut(bool)>>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `HoverTarget` doesn't implement `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = HoverTargetIvars]
    pub struct HoverTarget;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for HoverTarget {}

    impl HoverTarget {
        // SAFETY: matches `NSResponder`'s real `mouseEntered:`/`mouseExited:` signatures —
        // `event` is never read, only used to know *that* the pointer crossed the tracking area.
        // `.set()`, not `.push()`/`.pop()`: a push/pop stack only stays correct if every push is
        // matched by exactly one pop, which `NSTrackingArea`'s `.activeInKeyWindow` option can't
        // guarantee — the window losing key status (Cmd+Tab away, a click outside it) while
        // hovered skips `mouseExited:` entirely, leaving an unpaired push on the stack and the
        // pointing-hand cursor stuck until *something else* happens to pop it back off. `.set()`
        // instead assigns the cursor outright on every transition, so there's no accumulated state
        // for a missed event to desync into a stuck "cursor stays a pointer" state.
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &AnyObject) {
            NSCursor::pointingHandCursor().set();
            (self.ivars().on_change.borrow_mut())(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &AnyObject) {
            NSCursor::arrowCursor().set();
            (self.ivars().on_change.borrow_mut())(false);
        }
    }
);

impl HoverTarget {
    /// Attaches pointing-hand-cursor + hover-state tracking to `view` for as long as the returned
    /// `HoverTarget` stays alive (stash it next to the view, same lifetime contract `ActionTarget`
    /// already has). `on_change(true)`/`on_change(false)` fire on mouse enter/exit.
    pub fn attach(mtm: MainThreadMarker, view: &NSView, on_change: impl FnMut(bool) + 'static) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(HoverTargetIvars { on_change: RefCell::new(Box::new(on_change)) });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        let options = NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::InVisibleRect;

        // SAFETY: `owner` outlives the tracking area (the returned `HoverTarget` is what the
        // caller keeps alive alongside `view`, and the tracking area is removed automatically
        // when `view` itself is deallocated/removed from its superview).
        let tracking_area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), NSRect::default(), options, Some(&this), None)
        };
        view.addTrackingArea(&tracking_area);

        this
    }
}
