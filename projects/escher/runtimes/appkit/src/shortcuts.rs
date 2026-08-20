//! Global back/forward/refresh: a mouse side-button press or Cmd+[ / Cmd+] / Cmd+R anywhere in
//! the app, not just when the chrome bar's own buttons/address field have focus. Uses a local
//! (this-app-only, no Accessibility permission needed — unlike a *global* monitor) `NSEvent`
//! monitor, installed once per window.

use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadMarker;

use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags, NSEventType};

/// Kept alive for as long as the shortcuts should stay active — dropping it removes the monitor
/// (`NSEvent::removeMonitor:`), the same lifetime contract every other `attach`-style type in this
/// workspace already follows.
pub struct GlobalShortcuts {
    monitor: Option<Retained<AnyObject>>,
}

/// Mouse button numbers 3/4 are the de facto standard "back"/"forward" side buttons on virtually
/// every multi-button mouse — the same convention every major browser already honors.
const MOUSE_BACK_BUTTON: isize = 3;
const MOUSE_FORWARD_BUTTON: isize = 4;

impl GlobalShortcuts {
    pub fn install(_mtm: MainThreadMarker, on_back: impl Fn() + 'static, on_forward: impl Fn() + 'static, on_refresh: impl Fn() + 'static) -> Self {
        let mask = NSEventMask::KeyDown | NSEventMask::OtherMouseDown;

        let handler = block2::RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: the monitor contract guarantees a valid, non-null `NSEvent` for the
            // duration of this call.
            let event = unsafe { event.as_ref() };

            match event.r#type() {
                NSEventType::OtherMouseDown => match event.buttonNumber() {
                    MOUSE_BACK_BUTTON => on_back(),
                    MOUSE_FORWARD_BUTTON => on_forward(),
                    _ => {}
                },
                NSEventType::KeyDown if event.modifierFlags().contains(NSEventModifierFlags::Command) => {
                    match event.charactersIgnoringModifiers().map(|s| s.to_string()).as_deref() {
                        Some("[") => on_back(),
                        Some("]") => on_forward(),
                        Some("r") | Some("R") => on_refresh(),
                        _ => {}
                    }
                }
                _ => {}
            }

            // Passing the event through unchanged (not swallowing it) — a global back/forward/
            // refresh shortcut shouldn't stop the key/click from also doing whatever it would
            // have anyway (e.g. Cmd+R inside a focused address field editing normally too).
            event as *const NSEvent as *mut NSEvent
        });

        // SAFETY: `handler` is a real `block2::RcBlock`, sendable — it only captures `Fn` closures
        // (no arena/non-`Send` data).
        let monitor = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) };

        GlobalShortcuts { monitor }
    }
}

impl Drop for GlobalShortcuts {
    fn drop(&mut self) {
        if let Some(monitor) = self.monitor.take() {
            // SAFETY: `monitor` is exactly the token `addLocalMonitorForEventsMatchingMask:
            // handler:` returned, never shared elsewhere.
            unsafe { NSEvent::removeMonitor(&monitor) };
        }
    }
}
