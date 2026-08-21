//! A single reusable `NSObject` subclass that bridges AppKit's target-action mechanism (which
//! requires a real Objective-C object + selector, unlike a block-based callback) into a plain
//! Rust closure. One instance per interactive native control (`NSButton`/`NSTextField`) — created
//! once at spawn time in `surface.rs`'s `reconcile`, kept alive for as long as that control is
//! (stashed alongside it in `NativeNode`), never re-wired on a patch (only the closure's own
//! captured `NodePath`/outbox stay fixed; content updates go through the control directly).

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_foundation::{NSObject, NSObjectProtocol};

pub struct ActionTargetIvars {
    handler: RefCell<Box<dyn FnMut()>>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `ActionTarget` doesn't implement `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ActionTargetIvars]
    pub struct ActionTarget;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for ActionTarget {}

    impl ActionTarget {
        // SAFETY: matches the `(id)sender` signature every `NSControl` target-action selector is
        // invoked with — `sender` is never read, so its exact type doesn't matter here.
        #[unsafe(method(fire:))]
        fn fire(&self, _sender: &AnyObject) {
            (self.ivars().handler.borrow_mut())();
        }
    }
);

impl ActionTarget {
    pub fn new(mtm: MainThreadMarker, handler: impl FnMut() + 'static) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ActionTargetIvars { handler: RefCell::new(Box::new(handler)) });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}
