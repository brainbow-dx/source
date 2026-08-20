//! A plain `NSView` subclass whose only job is `isFlipped -> true`. AppKit's default (origin
//! bottom-left, y increasing upward) can't be changed on a stock `NSView` without subclassing.
//! Every container this surface creates uses this instead of plain `NSView`, so all of `surface.rs`'s
//! own layout math can assume one consistent coordinate convention (origin top-left, y increasing
//! downward) internally, regardless of whether the *outer* host window's own content view happens
//! to be flipped or not (that mismatch is handled once, at `AppKitSurface::attach`, the same way
//! `runtimes/os/src/macos/chrome.rs`'s original `attach` already had to).

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSAnimationContext, NSBezierPath, NSColor, NSCursor, NSEvent, NSTextFieldCell, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSString};

/// `fill`, when set, is a `(r, g, b)` triple (0-255). `AppKitSurface`'s own root container uses
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
/// displacement since `mouseDown:`. Below `CLICK_THRESHOLD` points counts as a click, past it
/// counts as a drag. Deliberately doesn't track/report live position during `mouseDragged:` (no
/// visual drop-indicator while dragging). Simplest correct version of "movable tabs," not the
/// fully polished one; a real drop-indicator is a reasonable follow-up, not attempted here.
/// Reports both outcomes as a `f64` (0.0 for a plain click, the total vertical displacement in
/// points for a drag) via `on_release`, which owns turning that into "select" vs. "moved N
/// positions" (needs the tab strip's own row-height bookkeeping, which lives with the app's tab
/// state, not in this view).
pub struct TabRowViewIvars {
    drag_start: RefCell<Option<NSPoint>>,
    on_release: RefCell<Box<dyn FnMut(f64)>>,
    selected: Cell<bool>,
    /// Active-row highlight color, `(r, g, b)` 0-255. Set once at construction from the current
    /// theme (see `AppKitSurface::set_theme`). `None` falls back to the system's own
    /// `selectedContentBackgroundColor`, same as before theming existed.
    highlight: Option<(f64, f64, f64)>,
    /// Whether the pointer is currently over this row. Used to draw a dimmer version of
    /// `highlight` on non-selected rows, so hovering *any* tab reads as interactive even before
    /// it's clicked.
    hovering: Cell<bool>,
}

/// How long a highlight fade takes. Subtle but smooth, not a snap and not a lazy crawl.
const HIGHLIGHT_FADE_SECONDS: f64 = 0.12;

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

        // SAFETY: matches `NSResponder`'s real `updateLayer` signature. Called by AppKit
        // whenever a layer-backed view needs to repaint its own backing layer directly, the
        // layer-backed counterpart to `drawRect:`. `apply_highlight` is what actually paints
        // (`layer.backgroundColor`, animated); this override just tells AppKit "yes, updateLayer
        // is how this view wants to be redrawn," matching the API `wantsUpdateLayer` promises.
        #[unsafe(method(wantsUpdateLayer))]
        fn wants_update_layer(&self) -> bool {
            true
        }

        // SAFETY: matches `NSResponder`'s real `mouseEntered:`/`mouseExited:` signatures.
        // `.set()`, not `.push()`/`.pop()`. See `hover.rs`'s own doc comment: `ActiveInKeyWindow`
        // tracking areas can skip `mouseExited:` when the window loses key status while still
        // hovered, leaving an unpaired push on the cursor stack and the pointing-hand cursor
        // stuck until something else happens to pop it back off.
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            NSCursor::pointingHandCursor().set();
            self.ivars().hovering.set(true);
            self.apply_highlight();
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            NSCursor::arrowCursor().set();
            self.ivars().hovering.set(false);
            self.apply_highlight();
        }
    }
);

/// A thin, always-present vertical strip at the tab strip's right edge. Drag it to resize the
/// sidebar (see `AppKitSurface::attach_sidebar`/`resize_handle`). Unlike `TabRowView`, reports the
/// live horizontal delta on every `mouseDragged:` tick, not just the total displacement at
/// release. A resize needs to track the pointer continuously, not just where it ended up.
/// Deliberately *not* part of the reconciled `Scaffold` tree at all: this doesn't vary with
/// `state.tabs` the way a drawn tree node would, it's surface chrome `AppKitSurface` owns
/// directly, the same way its own root view is.
///
/// `root` is reframed directly, synchronously, from inside `mouseDragged:` itself. Confirmed
/// live as a real fix for real jitter: routing every drag tick only through the `on_resize`
/// callback (which an app forwards into its own ECS event queue) meant the sidebar's actual frame
/// only ever moved whenever that engine's own scheduler next happened to run, and a live AppKit
/// mouse-drag runs its own nested tracking run loop that doesn't pump an engine's own (possibly
/// throttled) event loop at anything like the same steady rate, so the frame visibly lagged and
/// jumped in batches instead of tracking the pointer smoothly. `on_resize` still fires every tick,
/// same as before, for whatever state (`crate::bevy::TabStripState::width`, a webview's own
/// inset) needs to catch up. It's just no longer the thing that makes the drag *look* smooth.
pub struct SidebarResizeHandleIvars {
    drag_start: RefCell<Option<NSPoint>>,
    on_resize: RefCell<Box<dyn FnMut(f64)>>,
    root: Retained<NSView>,
    /// `None` (the default, before `set_fill_color` is ever called) draws nothing, leaving
    /// whatever's behind this view showing through. An unthemed handle otherwise reads as a
    /// stray, out-of-place gray/white strip wedged between the sidebar and the webview instead of
    /// a deliberate divider. `AppKitSurface::set_theme` sets this to
    /// `theme.background`, the same fill `FlippedView::set_fill_color` gives the sidebar's own
    /// root, so the strip reads as a continuation of the sidebar's edge rather than a gap in it.
    fill: Cell<Option<(f64, f64, f64)>>,
}

define_class!(
    // SAFETY: `NSView` has no subclassing requirements beyond what's overridden here;
    // `SidebarResizeHandle` doesn't implement `Drop`.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SidebarResizeHandleIvars]
    pub struct SidebarResizeHandle;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for SidebarResizeHandle {}

    impl SidebarResizeHandle {
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

        // SAFETY: matches `NSResponder`'s real `mouseDown:` signature.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            *self.ivars().drag_start.borrow_mut() = Some(point);
        }

        // SAFETY: matches `NSResponder`'s real `mouseDragged:` signature. Reports the delta since
        // the *previous* tick, not since `mouseDown:`. `drag_start` is rewritten to the current
        // point every call, rather than left alone the way `TabRowView::mouse_up`'s one-shot
        // `.take()` does, since the caller wants a running total it's already tracking, not a
        // final displacement.
        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let Some(start) = self.ivars().drag_start.borrow_mut().take() else { return };
            let point = self.convertPoint_fromView(event.locationInWindow(), None);
            let delta = point.x - start.x;
            *self.ivars().drag_start.borrow_mut() = Some(point);
            if delta == 0.0 {
                return;
            }

            // Reframes `root` (and this view, right past its new edge) immediately. See this
            // struct's own doc comment for why that can't wait on `on_resize`'s own event-queue
            // round trip without visibly jittering.
            let root = &self.ivars().root;
            let mut root_frame = root.frame();
            root_frame.size.width = (root_frame.size.width + delta).clamp(crate::surface::MIN_WIDTH, crate::surface::MAX_WIDTH);
            root.setFrame(root_frame);

            let mut own_frame = self.frame();
            own_frame.origin.x = root_frame.origin.x + root_frame.size.width;
            self.setFrame(own_frame);

            (self.ivars().on_resize.borrow_mut())(delta);
        }

        // SAFETY: matches `NSResponder`'s real `mouseUp:` signature.
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            *self.ivars().drag_start.borrow_mut() = None;
        }

        // SAFETY: matches `NSResponder`'s real `mouseEntered:`/`mouseExited:` signatures.
        // `.set()`, not `.push()`/`.pop()`. See `hover.rs`'s own doc comment: `ActiveInKeyWindow`
        // tracking areas can skip `mouseExited:` when the window loses key status while still
        // hovered, leaving an unpaired push on the cursor stack and the resize cursor stuck until
        // something else happens to pop it back off.
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            NSCursor::columnResizeCursor().set();
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            NSCursor::arrowCursor().set();
        }
    }
);

impl SidebarResizeHandle {
    /// `root` is the tab strip's own root view. Kept around so `mouseDragged:` can reframe it
    /// directly (see this struct's own doc comment for why).
    pub fn new(mtm: MainThreadMarker, frame: NSRect, root: Retained<NSView>) -> Retained<Self> {
        let this = Self::alloc(mtm)
            .set_ivars(SidebarResizeHandleIvars { drag_start: RefCell::new(None), on_resize: RefCell::new(Box::new(|_| {})), root, fill: Cell::new(None) });
        // SAFETY: `NSView`'s `initWithFrame:` has this exact signature.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };

        let options = NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveInKeyWindow | NSTrackingAreaOptions::InVisibleRect;
        // SAFETY: `this` (the tracking area's owner) outlives the tracking area. Both are torn
        // down together whenever `this` is deallocated/removed from its superview.
        let tracking_area =
            unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), NSRect::default(), options, Some(&this), None) };
        this.addTrackingArea(&tracking_area);

        this
    }

    /// Replaces the callback fired (with the horizontal delta in points, positive = rightward)
    /// on every `mouseDragged:` tick. Called fresh every draw, the same "rebuild the closure each
    /// tick" convention `crate::tabs::tab_strip`'s own `on_select`/`on_close`/etc. callbacks
    /// already use, so it can always capture whatever's currently live rather than something
    /// stale from whenever this view was first created.
    pub fn set_on_resize(&self, on_resize: impl FnMut(f64) + 'static) {
        *self.ivars().on_resize.borrow_mut() = Box::new(on_resize);
    }

    /// Whether a native drag is currently in progress. `drag_start` is `Some` from `mouseDown:`
    /// until `mouseUp:` clears it, so this doubles as that check. `AppKitSurface::reposition`
    /// uses this to skip re-applying a possibly-stale Bevy-side width mid-drag; see its own doc
    /// comment for why that matters.
    pub fn is_dragging(&self) -> bool {
        self.ivars().drag_start.borrow().is_some()
    }

    /// Sets (or clears, via `None`) this view's fill and repaints if it actually changed. Same
    /// contract as `FlippedView::set_fill_color`.
    pub fn set_fill_color(&self, color: Option<(u8, u8, u8)>) {
        let color = color.map(|(r, g, b)| (r as f64, g as f64, b as f64));
        if self.ivars().fill.replace(color) != color {
            self.setNeedsDisplay(true);
        }
    }
}

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
        // SAFETY: `this` (the tracking area's owner) outlives the tracking area. Both are torn
        // down together whenever `this` is deallocated/removed from its superview.
        let tracking_area =
            unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), NSRect::default(), options, Some(&this), None) };
        this.addTrackingArea(&tracking_area);

        // Layer-backed so `apply_highlight` can paint via `layer.backgroundColor` instead of a
        // manual `NSBezierPath` fill in `drawRect:`. A plain property set on a view's own
        // backing layer doesn't animate by default in AppKit (unlike a bare, standalone
        // `CALayer`), so the fade itself still needs `NSAnimationContext`; this is just what makes
        // there be a layer to fade at all. Also where the row's own rounding comes from: a
        // full-bleed, sharp-cornered fill running edge to edge in the sidebar read as a flat
        // colored block rather than a tab; every real tab treatment (including Edge's own
        // vertical-tabs mode) rounds the chip. `crate::tabs::TAB_STRIP_PADDING`'s own margin
        // around the whole strip is what keeps a rounded row from touching the sidebar's edges.
        this.setWantsLayer(true);
        if let Some(layer) = this.layer() {
            layer.setCornerRadius(6.0);
        }
        this.apply_highlight();

        this
    }

    /// Marks this row as the active tab (or not), fading only if the state actually changed.
    pub fn set_selected(&self, selected: bool) {
        if self.ivars().selected.replace(selected) != selected {
            self.apply_highlight();
        }
    }

    /// Fades `layer.backgroundColor` to whatever `selected`/`hovering` currently call for. The
    /// one place either state actually gets painted. `NSAnimationContext.runAnimationGroup` (not
    /// just setting the color directly) is what makes this animate at all: AppKit suppresses
    /// implicit `CALayer` actions on a view's own backing layer outside of an active animation
    /// context, unlike a bare Core Animation app, where `backgroundColor` would already animate
    /// on its own.
    fn apply_highlight(&self) {
        let Some(layer) = self.layer() else { return };

        let selected = self.ivars().selected.get();
        let hovering = self.ivars().hovering.get();
        let color = if !selected && !hovering {
            None
        } else {
            Some(match self.ivars().highlight {
                Some((r, g, b)) => {
                    // Much lower than a plain full-strength fill. A full-strength selected-tab
                    // highlight reads as too strong. `0.22` is a wash of color,
                    // enough to pick the active tab out at a glance without it looking like a
                    // solid block; hovering a non-selected row stays proportionally dimmer.
                    let alpha = if selected { 0.22 } else { 0.1 };
                    NSColor::colorWithSRGBRed_green_blue_alpha(r / 255.0, g / 255.0, b / 255.0, alpha)
                }
                None => NSColor::selectedContentBackgroundColor(),
            })
        };
        let cg_color = color.map(|color| color.CGColor());

        let animation = block2::RcBlock::new(move |context: NonNull<NSAnimationContext>| {
            // SAFETY: `NSAnimationContext.runAnimationGroup` guarantees a valid, non-null context
            // for the duration of this call.
            unsafe { context.as_ref() }.setDuration(HIGHLIGHT_FADE_SECONDS);
            layer.setBackgroundColor(cg_color.as_deref());
        });
        NSAnimationContext::runAnimationGroup(&animation);
    }
}

/// No actual state. Exists only so `alloc().set_ivars(..)` produces the `PartialInit` handle
/// `objc2`'s `super(..)`-call machinery requires for a designated-initializer override, the same
/// shape every other custom class in this file already follows (see `ActionTarget` in
/// `crate::action` for the same empty-ivars-purely-for-init pattern).
pub struct VerticallyCenteredTextFieldCellIvars;

define_class!(
    // SAFETY: `NSTextFieldCell` has no subclassing requirements beyond what's overridden here;
    // `VerticallyCenteredTextFieldCell` doesn't implement `Drop`.
    #[unsafe(super = NSTextFieldCell)]
    #[thread_kind = MainThreadOnly]
    #[ivars = VerticallyCenteredTextFieldCellIvars]
    pub struct VerticallyCenteredTextFieldCell;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for VerticallyCenteredTextFieldCell {}

    impl VerticallyCenteredTextFieldCell {
        // A plain `NSTextFieldCell` draws its text flush with the top of its own bounds whenever
        // its cell is taller than one line of text. The normal case here, since every toolbar
        // control shares one `Flex`-stretched row height rather than sizing itself to its content.
        // Auto Layout callers dodge this by giving the field an intrinsic-height constraint
        // instead; this surface positions everything with plain frames, so the standard AppKit fix
        // is this cell override instead: centering `drawingRectForBounds:`'s returned rect within
        // whatever bounds it's asked to draw in.
        //
        // SAFETY: matches `NSCell`'s real `drawingRectForBounds:` signature (`NSRect -> NSRect`).
        #[unsafe(method(drawingRectForBounds:))]
        fn drawing_rect_for_bounds(&self, bounds: NSRect) -> NSRect {
            self.centered_rect(unsafe { msg_send![super(self), drawingRectForBounds: bounds] })
        }

        // `drawingRectForBounds:` alone centers the *static* display (an unfocused field showing
        // its placeholder or a committed value) but not the live field editor AppKit installs the
        // instant this field becomes first responder. Without this, actively-edited text
        // renders top-aligned regardless of the override above; only the static display is
        // actually centered by it. `titleRectForBounds:` is what the field editor
        // itself gets positioned against, so it needs the identical centering, not just a call
        // through to the override above. A plain cell's own default `titleRectForBounds:` doesn't
        // consult `drawingRectForBounds:` on its own.
        //
        // SAFETY: matches `NSCell`'s real `titleRectForBounds:` signature (`NSRect -> NSRect`).
        #[unsafe(method(titleRectForBounds:))]
        fn title_rect_for_bounds(&self, bounds: NSRect) -> NSRect {
            self.centered_rect(unsafe { msg_send![super(self), titleRectForBounds: bounds] })
        }
    }
);

/// Horizontal breathing room inside the field's own bounds. Without it, text/the field editor's
/// cursor sits flush against the rounded pill's own edge (see `AppKitSurface`'s `NodeKind::
/// TextField` spawn arm for the pill itself), which reads as cramped rather than a real padded
/// input the way every other browser's address bar is.
const HORIZONTAL_INSET: f64 = 10.0;

impl VerticallyCenteredTextFieldCell {
    fn centered_rect(&self, mut rect: NSRect) -> NSRect {
        let text_size = self.cellSizeForBounds(rect);
        let height_delta = rect.size.height - text_size.height;
        if height_delta > 0.0 {
            rect.size.height -= height_delta;
            rect.origin.y += height_delta / 2.0;
        }
        rect.origin.x += HORIZONTAL_INSET;
        rect.size.width -= HORIZONTAL_INSET * 2.0;
        rect
    }
}

impl VerticallyCenteredTextFieldCell {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(VerticallyCenteredTextFieldCellIvars);
        // SAFETY: `NSCell`'s designated initializer, `initTextCell:`, has this exact signature.
        unsafe { msg_send![super(this), initTextCell: &*NSString::from_str("")] }
    }
}
