//! A plain `NSView` subclass whose only job is `isFlipped -> true` — AppKit's default (origin
//! bottom-left, y increasing upward) can't be changed on a stock `NSView` without subclassing.
//! Every container this surface creates uses this instead of plain `NSView`, so all of `surface.rs`'s
//! own layout math can assume one consistent coordinate convention (origin top-left, y increasing
//! downward) internally, regardless of whether the *outer* host window's own content view happens
//! to be flipped or not (that mismatch is handled once, at `AppKitSurface::attach`, the same way
//! `runtimes/os/src/macos/chrome.rs`'s original `attach` already had to).

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBezierPath, NSColor, NSCursor, NSEvent, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect};

/// `fill`, when set, is a `(r, g, b)` triple (0-255) — `AppKitSurface`'s own root container uses
/// this to paint a themed background instead of showing through to whatever's behind it (the
/// window's default appearance), so a toolbar/tab-strip surface can match a styleguide instead of
/// just inheriting the OS chrome color. `None` (the default) paints nothing, same as before this
/// existed.
pub struct FlippedViewIvars {
    fill: Cell<Option<(f64, f64, f64)>>,
}

define_class!(
    // SAFETY: `NSView` has no subclassing requirements beyond what's overridden here;
    // `FlippedView` doesn't implement `Drop`.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FlippedViewIvars]
    pub struct FlippedView;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for FlippedView {}

    impl FlippedView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        // SAFETY: matches `NSView`'s real `drawRect:` signature.
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            if let Some((r, g, b)) = self.ivars().fill.get() {
                NSColor::colorWithSRGBRed_green_blue_alpha(r / 255.0, g / 255.0, b / 255.0, 1.0).set();
                NSBezierPath::fillRect(self.bounds());
            }
        }
    }
);

impl FlippedView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FlippedViewIvars { fill: Cell::new(None) });
        // SAFETY: `NSView`'s `initWithFrame:` has this exact signature.
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    /// Sets (or clears, via `None`) this view's background fill and repaints if it actually
    /// changed. `(r, g, b)` is 0-255, matching `escher_styleguide::Styleguide::color`'s shape.
    pub fn set_fill_color(&self, color: Option<(u8, u8, u8)>) {
        let color = color.map(|(r, g, b)| (r as f64, g as f64, b as f64));
        if self.ivars().fill.replace(color) != color {
            self.setNeedsDisplay(true);
        }
    }
}

/// A tab-strip row: click-to-select or drag-to-reorder, disambiguated at `mouseUp:` by total
/// displacement since `mouseDown:` — below `CLICK_THRESHOLD` points counts as a click, past it
/// counts as a drag. Deliberately doesn't track/report live position during `mouseDragged:` (no
/// visual drop-indicator while dragging) — simplest correct version of "movable tabs," not the
/// fully polished one; a real drop-indicator is a reasonable follow-up, not attempted here.
/// Reports both outcomes as a `f64` (0.0 for a plain click, the total vertical displacement in
/// points for a drag) via `on_release`, which owns turning that into "select" vs. "moved N
/// positions" (needs the tab strip's own row-height bookkeeping, which lives with the app's tab
/// state, not in this view).
pub struct TabRowViewIvars {
    drag_start: RefCell<Option<NSPoint>>,
    on_release: RefCell<Box<dyn FnMut(f64)>>,
    selected: Cell<bool>,
    /// Active-row highlight color, `(r, g, b)` 0-255 — set once at construction from the current
    /// theme (see `AppKitSurface::set_theme`). `None` falls back to the system's own
    /// `selectedContentBackgroundColor`, same as before theming existed.
    highlight: Option<(f64, f64, f64)>,
    /// Whether the pointer is currently over this row — a plain instant on/off (not an eased
    /// animation; see `escher-appkit`'s own doc notes on why real transitions were deliberately
    /// skipped for this pass) used to draw a dimmer version of `highlight` on non-selected rows,
    /// so hovering *any* tab reads as interactive even before it's clicked.
    hovering: Cell<bool>,
}

const CLICK_THRESHOLD: f64 = 4.0;

define_class!(
    // SAFETY: `NSView` has no subclassing requirements beyond what's overridden here;
    // `TabRowView` doesn't implement `Drop`.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = TabRowViewIvars]
    pub struct TabRowView;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for TabRowView {}

    impl TabRowView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        // SAFETY: matches `NSResponder`'s real `mouseDown:` signature.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            *self.ivars().drag_start.borrow_mut() = Some(point);
        }

        // SAFETY: matches `NSResponder`'s real `mouseUp:` signature.
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let Some(start) = self.ivars().drag_start.borrow_mut().take() else { return };
            let end = self.convertPoint_fromView(event.locationInWindow(), None);
            let delta = end.y - start.y;
            let reported = if delta.abs() < CLICK_THRESHOLD { 0.0 } else { delta };
            (self.ivars().on_release.borrow_mut())(reported);
        }

        // SAFETY: matches `NSView`'s real `drawRect:` signature. Not layer-backed (no `CALayer`
        // dependency pulled in for this) — a plain fill via `NSBezierPath`, the same mechanism any
        // non-layer-backed `NSView` subclass uses to paint itself.
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let selected = self.ivars().selected.get();
            let hovering = self.ivars().hovering.get();
            if !selected && !hovering {
                return;
            }
            match self.ivars().highlight {
                Some((r, g, b)) => {
                    // Hovering a non-selected row draws at a third of the strength of the real
                    // selected-tab highlight — enough to read as "interactive" without being
                    // mistaken for the actually-active tab.
                    let alpha = if selected { 1.0 } else { 0.35 };
                    NSColor::colorWithSRGBRed_green_blue_alpha(r / 255.0, g / 255.0, b / 255.0, alpha).set();
                }
                None => NSColor::selectedContentBackgroundColor().set(),
            }
            NSBezierPath::fillRect(self.bounds());
        }

        // SAFETY: matches `NSResponder`'s real `mouseEntered:`/`mouseExited:` signatures.
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            NSCursor::pointingHandCursor().push();
            self.ivars().hovering.set(true);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            NSCursor::pointingHandCursor().pop();
            self.ivars().hovering.set(false);
            self.setNeedsDisplay(true);
        }
    }
);

impl TabRowView {
    pub fn new(mtm: MainThreadMarker, frame: NSRect, on_release: impl FnMut(f64) + 'static, highlight: Option<(u8, u8, u8)>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TabRowViewIvars {
            drag_start: RefCell::new(None),
            on_release: RefCell::new(Box::new(on_release)),
            selected: Cell::new(false),
            highlight: highlight.map(|(r, g, b)| (r as f64, g as f64, b as f64)),
            hovering: Cell::new(false),
        });
        // SAFETY: `NSView`'s `initWithFrame:` has this exact signature.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };

        let options = NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::InVisibleRect;
        // SAFETY: `this` (the tracking area's owner) outlives the tracking area — both are torn
        // down together whenever `this` is deallocated/removed from its superview.
        let tracking_area =
            unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), NSRect::default(), options, Some(&this), None) };
        this.addTrackingArea(&tracking_area);

        this
    }

    /// Marks this row as the active tab (or not), repainting only if the state actually changed.
    pub fn set_selected(&self, selected: bool) {
        if self.ivars().selected.replace(selected) != selected {
            self.setNeedsDisplay(true);
        }
    }
}
