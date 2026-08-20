//! Configurable global shortcuts: any number of Cmd+<key> combos or mouse side-button presses,
//! anywhere in the app, not just when some specific control has focus. Uses a local (this-app-
//! only, no Accessibility permission needed — unlike a *global* monitor) `NSEvent` monitor, one
//! per window, installed once for the whole binding set rather than once per binding.
//!
//! Data-driven (`Vec<(Shortcut, Box<dyn Fn()>)>`) rather than fixed named parameters, specifically
//! so the binding set can come from somewhere configurable later (Escher's app-state manager,
//! eventually) instead of being hardcoded per caller forever — today's only caller
//! (`bevy.rs`'s `install_global_shortcuts`) still hardcodes its own defaults, but the API itself no
//! longer forces that.

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

/// One configurable trigger — either a Cmd+<key> combo or one of the two de facto standard mouse
/// side buttons (button numbers 3/4, the same convention every major browser already honors).
/// Deliberately minimal (Command-only, no Shift/Option/Control combinations) — extend when a real
/// binding actually needs more, not speculatively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    /// Matched case-insensitively against `charactersIgnoringModifiers` — register lowercase
    /// (`CommandKey('r')`), not `CommandKey('R')`.
    CommandKey(char),
    MouseBack,
    MouseForward,
}

const MOUSE_BACK_BUTTON: isize = 3;
const MOUSE_FORWARD_BUTTON: isize = 4;

impl GlobalShortcuts {
    /// Installs every `(Shortcut, action)` binding as a single monitor — a large binding set costs
    /// the same as a small one. Multiple bindings may share the same `Shortcut` (not deduplicated);
    /// all of their actions fire.
    pub fn install(_mtm: MainThreadMarker, bindings: Vec<(Shortcut, Box<dyn Fn()>)>) -> Self {
        let mask = NSEventMask::KeyDown | NSEventMask::OtherMouseDown;

        let handler = block2::RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: the monitor contract guarantees a valid, non-null `NSEvent` for the
            // duration of this call.
            let event = unsafe { event.as_ref() };

            let triggered = match event.r#type() {
                NSEventType::OtherMouseDown => match event.buttonNumber() {
                    MOUSE_BACK_BUTTON => Some(Shortcut::MouseBack),
                    MOUSE_FORWARD_BUTTON => Some(Shortcut::MouseForward),
                    _ => None,
                },
                NSEventType::KeyDown if event.modifierFlags().contains(NSEventModifierFlags::Command) => event
                    .charactersIgnoringModifiers()
                    .map(|s| s.to_string())
                    .and_then(|s| s.chars().next())
                    .map(|key| Shortcut::CommandKey(key.to_ascii_lowercase())),
                _ => None,
            };

            if let Some(triggered) = triggered {
                for (shortcut, action) in &bindings {
                    if *shortcut == triggered {
                        action();
                    }
                }
            }

            // Passing the event through unchanged (not swallowing it) — a global shortcut
            // shouldn't stop the key/click from also doing whatever it would have anyway (e.g.
            // Cmd+R inside a focused address field editing normally too).
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
