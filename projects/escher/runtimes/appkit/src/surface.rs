//! Renders an `escher_core::Scaffold` tree as real, native AppKit views — the AppKit analog of
//! `escher-bevy`'s `BevySurface`/`escher-web`'s DOM surface. Unlike those (which fully tear down
//! and rebuild their whole native tree every draw — fine for a Bevy UI node or a `<div>`, fatal
//! for an `NSTextField` a user might be actively typing into), this surface reconciles: each node
//! gets a stable identity (`escher_core::scaffold::NodePath`, the same `(TypeId, usize)` slot-key
//! chain every surface already has available) that survives across draws, so an unchanged native
//! object is patched in place — never destroyed and recreated — and only nodes whose path
//! genuinely disappears get torn down.
//!
//! Two node "shapes" get special, atomic treatment instead of the generic container-recursion
//! every other node gets: a node carrying a `Button` element becomes one `NSButton` (reading
//! `.label`/`.disabled` directly off the element, not by recursing into whatever child slot
//! `Button::draw()` happens to produce — that child exists for surfaces that *do* want to recurse
//! into it, like Terminal; this one doesn't need to). Likewise `Input<String>` becomes one
//! `NSTextField`. Everything else with content and no children becomes a plain text label;
//! everything else becomes a plain container view children lay out inside of.
//!
//! Native callbacks (a button click, a text field's Return-commit) can't safely hold a reference
//! into the `Scaffold` tree that registered their handler — that tree's whole arena is dropped the
//! moment the next `draw()` call replaces it, and a click might not happen until many draws later
//! (or never again). So a callback firing does the minimum possible: push `(NodePath, NativeEvent)`
//! onto `self.outbox`, a plain `'static` queue. Real dispatch into a node's actual handler closures
//! only happens at the *start* of the next `draw()` call, against the freshly-built, still-valid
//! tree, via `Scaffold::get_at_path` — never against a previous draw's arena. If a path no longer
//! resolves (the tree changed shape in the meantime), the event is silently dropped.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bumpalo::Bump;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, Message};

use objc2_app_kit::{NSButton, NSBezelStyle, NSColor, NSFont, NSImageView, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use raw_window_handle::RawWindowHandle;

use escher_core::element::{Button, Input};
use escher_core::event::{ClickEvent, SubmitEvent};
use escher_core::scaffold::{NodePath, Scaffold};
use escher_core::style::{Flex, FlexDirection, Gap, Padding, Property, Size, Value};

use escher_os::OsError;

use crate::action::ActionTarget;
use crate::favicon::FaviconCache;
use crate::views::FlippedView;

/// A node's currently-live native representation, keyed by `NodePath` in `AppKitSurface::nodes`.
/// The `ActionTarget` fields exist purely to keep the target-action bridge alive for as long as
/// the control is — never read again after `spawn`, same lifetime contract `ChromeBar::events`
/// used to have with its Swift-side counterpart.
enum NativeNode {
    Container(Retained<NSView>),
    Button {
        view: Retained<NSButton>,
        _target: Retained<ActionTarget>,
        /// `None` on an unthemed surface (nothing to hover-tint into) — see `crate::hover`.
        _hover: Option<Retained<crate::hover::HoverTarget>>,
    },
    TextField {
        view: Retained<NSTextField>,
        _target: Retained<ActionTarget>,
    },
    Label(Retained<NSTextField>),
    Image(Retained<NSImageView>),
    /// A tab-strip row — see `crate::views::TabRowView`'s own doc comment for the click-vs-drag
    /// disambiguation it implements. Still a plain container for layout purposes (its favicon/
    /// title/close-button children recurse normally, see `layout_children`); the only difference
    /// from `Container` is which native class backs it.
    Row(Retained<crate::views::TabRowView>),
}

impl NativeNode {
    fn as_view(&self) -> &NSView {
        match self {
            NativeNode::Container(v) => v,
            NativeNode::Button { view, .. } => view,
            NativeNode::TextField { view, .. } => view,
            NativeNode::Label(v) => v,
            NativeNode::Image(v) => v,
            NativeNode::Row(v) => v,
        }
    }
}

/// What a native callback pushes onto `AppKitSurface::outbox` — deliberately tiny and `'static`,
/// see this module's own doc comment for why it can't be anything richer (no arena references).
#[derive(Debug, Clone)]
pub enum NativeEvent {
    Activated,
    Submitted(String),
    /// `0.0` for a plain click (select), any other value for a completed drag (that many points
    /// of vertical displacement) — see `TabRowView`'s own doc comment.
    RowReleased(f64),
}

/// Dispatched into a tab-row node's handlers on `NativeEvent::RowReleased` — see that variant's
/// own doc comment for the click(`0.0`)-vs-drag(anything else) meaning of the payload. Local to
/// this crate, not `escher_core`, same reasoning as `runtimes/terminal`'s own local `ClickEvent`:
/// this is a genuinely AppKit-tab-strip-specific interaction, not a general cross-surface concept.
#[derive(Debug, Clone, Copy)]
pub struct TabRowReleased(pub f64);

/// Which kind of native object a `Scaffold` node should become, decided once per node per draw by
/// `classify` — carries just enough extracted data (`get_element::<E>()` copies, not `&Scaffold`
/// borrows) to spawn/patch without re-walking the tree.
enum NodeKind {
    Container,
    Button { label: String, disabled: bool },
    TextField { value: String, placeholder: Option<String> },
    Label(String),
    Favicon { host: String },
    TabRow { selected: bool },
}

/// Marks a node as "render the cached favicon for this host" — surface-specific (favicon caching
/// is an AppKit-surface concern, not something `escher-core` needs to know about), read via
/// `get_element::<FaviconImage>()` the same way `Button`/`Input` are.
#[derive(Debug, Clone)]
pub struct FaviconImage {
    pub host: String,
}

impl escher_core::element::Element for FaviconImage {}

/// Marks a node as a tab-strip row — see `NodeKind::TabRow`/`NativeNode::Row`. `selected` is the
/// only data it carries; a row's actual tab identity lives in the closure `crate::tabs::tab_strip`
/// gives each row's `.with_handler::<TabRowReleased>(..)`, not in this marker.
#[derive(Debug, Clone, Default)]
pub struct TabRowMarker {
    pub selected: bool,
}

impl escher_core::element::Element for TabRowMarker {}

fn classify(node: &Scaffold) -> NodeKind {
    if let Some(button) = node.get_element::<Button>() {
        return NodeKind::Button { label: button.label.clone(), disabled: button.disabled };
    }

    if let Some(input) = node.get_element::<Input<String>>() {
        return NodeKind::TextField { value: input.value.clone(), placeholder: input.placeholder.clone() };
    }

    if let Some(favicon) = node.get_element::<FaviconImage>() {
        return NodeKind::Favicon { host: favicon.host.clone() };
    }

    if let Some(marker) = node.get_element::<TabRowMarker>() {
        return NodeKind::TabRow { selected: marker.selected };
    }

    if let Some(content) = node.get_content()
        && node.get_slots().is_empty()
    {
        return NodeKind::Label(content.to_string());
    }

    NodeKind::Container
}

fn flex_direction(styles: &escher_core::style::StyleSheet) -> FlexDirection {
    styles.iter().flat_map(|(_, values)| values).find_map(|property| match property {
        Property::FlexDirection(direction) => Some(*direction),
        _ => None,
    }).unwrap_or_default()
}

fn gap_px(styles: &escher_core::style::StyleSheet) -> f64 {
    styles.iter().flat_map(|(_, values)| values).find_map(|property| match property {
        Property::Gap(Gap(value)) => Some(px(*value)),
        _ => None,
    }).unwrap_or(0.0)
}

/// `(top, right, bottom, left)` — `Padding`'s own `Edge` targets one side per call (or `All` for
/// every side at once), so several `Padding` entries can coexist in the same style sheet; this
/// folds all of them down to one inset per side, last-write-wins per side (matches how every other
/// surface's own `sum_insets`/`apply_edge`-style folding already treats repeated edge styles).
fn padding_insets(styles: &escher_core::style::StyleSheet) -> (f64, f64, f64, f64) {
    let mut insets = (0.0, 0.0, 0.0, 0.0);
    for property in styles.iter().flat_map(|(_, values)| values) {
        if let Property::Padding(Padding(edge, value)) = property {
            let px = px(*value);
            match edge {
                escher_core::style::Edge::All => insets = (px, px, px, px),
                escher_core::style::Edge::Top => insets.0 = px,
                escher_core::style::Edge::Right => insets.1 = px,
                escher_core::style::Edge::Bottom => insets.2 = px,
                escher_core::style::Edge::Left => insets.3 = px,
                escher_core::style::Edge::None => {}
            }
        }
    }
    insets
}

fn flex_weight(styles: &escher_core::style::StyleSheet) -> Option<f64> {
    styles.iter().flat_map(|(_, values)| values).find_map(|property| match property {
        Property::Flex(Flex(unit)) => Some(unit.0),
        _ => None,
    })
}

/// The explicit main-axis `Size` a node asked for, if any — width for a `Row` parent, height for
/// a `Column` one. `Value::Fill`/`Value::Percent`/`Value::Auto` aren't resolved here (no core
/// consumer sets them today outside `Px`) — treated the same as "no explicit size," falling
/// through to `Flex` or intrinsic sizing instead. Worth revisiting if a future composition needs
/// them.
fn explicit_main_px(styles: &escher_core::style::StyleSheet, direction: FlexDirection) -> Option<f64> {
    styles.iter().flat_map(|(_, values)| values).find_map(|property| match property {
        Property::Size(Size(width, height, _)) => {
            let value = if direction == FlexDirection::Row { width } else { height };
            if let Value::Px(unit) = value { Some(unit.0) } else { None }
        }
        _ => None,
    })
}

fn explicit_cross_px(styles: &escher_core::style::StyleSheet, direction: FlexDirection) -> Option<f64> {
    styles.iter().flat_map(|(_, values)| values).find_map(|property| match property {
        Property::Size(Size(width, height, _)) => {
            let value = if direction == FlexDirection::Row { height } else { width };
            if let Value::Px(unit) = value { Some(unit.0) } else { None }
        }
        _ => None,
    })
}

fn px(value: Value) -> f64 {
    match value {
        Value::Px(unit) | Value::Fill(unit) | Value::Percent(unit) => unit.0,
        Value::Auto => 0.0,
    }
}

/// Where an `AppKitSurface`'s root view sits relative to its host, recomputed against the host's
/// *current* frame at the top of every `draw()` call (see `AppKitSurface::reposition`) rather than
/// left to AppKit's own autoresizing masks — masks work fine until a *flipped* superview enters
/// the picture, at which point which margin flag means "pinned to the visual top" versus "pinned
/// to the visual bottom" stops being documented anywhere reliable, and got it wrong in practice
/// (this is why the toolbar/tab strip used to drift on window resize: the mask that was
/// supposed to keep them pinned instead let their origin float). Recomputing the frame outright
/// every draw sidesteps the ambiguity entirely — same one-time math `attach`/`attach_sidebar` used
/// to do once at construction, just re-run continuously against `AppKitSurface::draw`'s own
/// already-every-tick cadence, which costs nothing extra it wasn't already paying.
enum Pin {
    /// Full width, `height` points tall, flush with the host's top edge.
    Top { height: f64 },
    /// Fixed `width`, flush with the host's left edge, starting `top_offset` points down (below a
    /// `Pin::Top` surface attached separately) and filling the remaining height.
    Left { width: f64, top_offset: f64 },
}

/// The minimal set of colors + text sizes an `AppKitSurface` themes itself with: `background`
/// (this surface's own root fill), `surface` (a control's own background — e.g. the address field
/// — one step lighter than `background` so it reads as a raised control instead of blending into
/// the bar behind it), `accent` (active tab-row highlight), `text` (labels/text fields/button
/// titles), `ui_text_size` (button glyphs — back/forward/refresh/hamburger/close, generally wants
/// to read a bit larger than body copy), and `body_text_size` (labels/text field content). Two
/// sizes, not one, so a themed surface can size icon glyphs and body text independently while
/// still guaranteeing every text-bearing control actually gets *some* explicit, consistent size
/// instead of silently falling back to whatever the default system font happens to be (previously:
/// only buttons had an explicit size, labels/fields didn't, so the two could drift out of visual
/// sync depending on the OS's own default). See `escher_styleguide::Styleguide` for where these
/// normally come from; this type exists so `surface.rs` doesn't need to depend on that crate
/// directly. `None` on `AppKitSurface` (the default) means "untouched, system default appearance"
/// — the same look this surface had before theming existed.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: (u8, u8, u8),
    pub surface: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub text: (u8, u8, u8),
    pub ui_text_size: f64,
    pub body_text_size: f64,
}

fn rgb_color((r, g, b): (u8, u8, u8)) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0)
}

/// Attached at the top (or side) of some window, reconciles+renders a `Scaffold` tree into it
/// every `draw()` call. Replaces `runtimes/os/src/macos/chrome.rs`'s Swift-hosted toolbar — see
/// this module's own doc comment for the reconciliation contract, and `escher_chalk::toolbar`/
/// `crate::tabs` for the two concrete compositions built on top of this.
pub struct AppKitSurface {
    root: Retained<NSView>,
    /// The window content view this surface is attached to — kept around (not just borrowed at
    /// `attach` time) so `reposition` can re-read its *current* frame every draw; see `Pin`'s own
    /// doc comment for why that's needed instead of a one-time autoresizing mask.
    host: Retained<NSView>,
    /// Same object as `root`, kept typed so `set_theme` can call `FlippedView::set_fill_color` on
    /// it — `root` itself is upcast to plain `NSView` since every other use site (layout, event
    /// dispatch) only needs generic `NSView` methods.
    root_flipped: Retained<crate::views::FlippedView>,
    pin: Pin,
    theme: Option<Theme>,
    /// Called (if set) the instant a native callback — a button click, a text field submit, a tab
    /// row release — pushes onto `outbox`, *before* that event is ever dispatched. Exists because
    /// these are raw Cocoa target-action/mouse callbacks on views this crate created directly
    /// (not routed through winit's own window-event mapping at all), so without this, a click here
    /// only gets noticed whenever Bevy's `Update` schedule next happens to run — which, under
    /// `WinitSettings::desktop_app()`'s reactive throttling, can be seconds later. Same fix shape
    /// as `escher_bevy::terminal::spawn_signal_watcher`/`spawn_input_watcher` use for signals and
    /// keyboard input: wake the event loop explicitly rather than hoping it notices on its own. See
    /// `set_wake_callback`.
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
    nodes: HashMap<NodePath, NativeNode>,
    outbox: Arc<Mutex<Vec<(NodePath, NativeEvent)>>>,
    mtm: MainThreadMarker,
    pub favicons: FaviconCache,
}

impl AppKitSurface {
    /// Attaches an empty, flipped container to the top of `parent`'s window, `height` points
    /// tall, full width, pinned to the top edge and kept frontmost — geometry/z-order/flip
    /// handling ported from `runtimes/os/src/macos/chrome.rs`'s original `attach` (now deleted).
    /// No content yet; the first `draw()` call builds it.
    pub fn attach(parent: RawWindowHandle, height: f64) -> Result<Self, OsError> {
        Self::attach_pinned(parent, Pin::Top { height })
    }

    /// Same idea as `attach`, but pinned to the *left* edge with a fixed `width`, starting
    /// `top_offset` points down from the very top (e.g. below a toolbar attached separately via
    /// plain `attach`) — for the tab strip, which sits beside the toolbar rather than above the
    /// page.
    pub fn attach_sidebar(parent: RawWindowHandle, width: f64, top_offset: f64) -> Result<Self, OsError> {
        Self::attach_pinned(parent, Pin::Left { width, top_offset })
    }

    fn attach_pinned(parent: RawWindowHandle, pin: Pin) -> Result<Self, OsError> {
        let RawWindowHandle::AppKit(appkit_handle) = parent else {
            return Err(OsError::Unsupported);
        };

        let mtm = MainThreadMarker::new().ok_or(OsError::NotOnMainThread)?;

        // SAFETY: the caller is responsible for `parent` staying valid for as long as the
        // returned `AppKitSurface` is alive — the same contract every other consumer of a
        // `RawWindowHandle` in this workspace already places on itself.
        let ns_view: &NSView = unsafe { appkit_handle.ns_view.cast().as_ref() };

        let root = FlippedView::new(mtm, NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)));

        const NS_WINDOW_ABOVE: isize = 1;
        unsafe {
            let _: () = objc2::msg_send![ns_view, addSubview: &*root, positioned: NS_WINDOW_ABOVE, relativeTo: std::ptr::null::<NSView>()];
        }

        let surface = AppKitSurface {
            root: Retained::into_super(Retained::clone(&root)),
            root_flipped: root,
            host: ns_view.retain(),
            pin,
            theme: None,
            wake: None,
            nodes: HashMap::new(),
            outbox: Arc::new(Mutex::new(Vec::new())),
            mtm,
            favicons: FaviconCache::new(),
        };
        surface.reposition();

        Ok(surface)
    }

    /// Recomputes `root`'s frame from `host`'s *current* frame and flip state and applies it — see
    /// `Pin`'s own doc comment for why this replaces relying on an autoresizing mask.
    fn reposition(&self) {
        let host_frame = self.host.frame();
        let host_is_flipped = self.host.isFlipped();

        let frame = match self.pin {
            Pin::Top { height } => {
                // Which edge "top" is depends on whether the *host* (not this surface's own
                // always-flipped root) is itself flipped.
                let origin_y = if host_is_flipped { 0.0 } else { (host_frame.size.height - height).max(0.0) };
                NSRect::new(NSPoint::new(0.0, origin_y), NSSize::new(host_frame.size.width, height))
            }
            Pin::Left { width, top_offset } => {
                let origin_y = if host_is_flipped { top_offset } else { 0.0 };
                let height = (host_frame.size.height - top_offset).max(0.0);
                NSRect::new(NSPoint::new(0.0, origin_y), NSSize::new(width, height))
            }
        };

        self.root.setFrame(frame);
    }

    /// Changes a `Pin::Left` surface's width in place — e.g. collapsing/expanding a tab strip
    /// after it's already attached. A no-op for a `Pin::Top` surface (its width always tracks the
    /// host's own, there's nothing meaningful to override). Takes effect on the very next `draw()`
    /// (via `reposition`), or immediately if called between draws — either way, callers that also
    /// need to re-inset content sharing the same edge (a webview sitting beside this sidebar, say)
    /// still need to update that separately; this only owns the sidebar's own frame.
    pub fn set_width(&mut self, width: f64) {
        if let Pin::Left { width: current, .. } = &mut self.pin {
            *current = width;
            self.reposition();
        }
    }

    /// Applies a theme: paints `root`'s background immediately, and remembers `accent`/`text` for
    /// nodes created from here on (existing tab rows/labels don't retroactively repaint — call
    /// this before the surface's first `draw()`, which is the only time this crate itself calls
    /// it today). See `Theme`'s own doc comment for why it's only these three colors.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = Some(theme);
        self.root_flipped.set_fill_color(Some(theme.background));
    }

    /// Registers a callback fired the instant any native control this surface owns is activated —
    /// see `wake`'s own doc comment for why this exists. Applies to nodes created from here on;
    /// call before the first `draw()`, same as `set_theme`.
    pub fn set_wake_callback(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.wake = Some(wake);
    }

    /// Builds a fresh tree from `draw_fn`, dispatches any pending native callbacks against it,
    /// reconciles the native view tree to match, and lays it out. See this module's doc comment
    /// for the full contract.
    pub fn draw<F>(&mut self, draw_fn: F)
    where
        F: for<'ctx> FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>,
    {
        self.reposition();

        let bump = Bump::new();
        let tree = draw_fn(Scaffold::new_in(&bump));

        self.dispatch_pending(&tree);

        let mut visited: HashSet<NodePath> = HashSet::new();
        let bounds = self.root.bounds();
        let root_view = Retained::clone(&self.root);
        self.layout_children(&tree, &root_view, bounds, &NodePath::root(), &mut visited);

        let stale: Vec<NodePath> = self.nodes.keys().filter(|path| !visited.contains(*path)).cloned().collect();
        for path in stale {
            if let Some(native) = self.nodes.remove(&path) {
                native.as_view().removeFromSuperview();
            }
        }
    }

    fn dispatch_pending(&self, tree: &Scaffold) {
        let pending = std::mem::take(&mut *self.outbox.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));

        for (path, event) in pending {
            let Some(node) = tree.get_at_path(&path) else { continue };

            match event {
                NativeEvent::Activated => node.get_handlers().exec::<ClickEvent>(&ClickEvent),
                NativeEvent::Submitted(text) => node.get_handlers().exec::<SubmitEvent>(&SubmitEvent(text)),
                NativeEvent::RowReleased(delta) => node.get_handlers().exec::<TabRowReleased>(&TabRowReleased(delta)),
            }
        }
    }

    fn layout_children<'ctx>(&mut self, scaffold: &Scaffold<'ctx>, parent_view: &NSView, available_rect: NSRect, base_path: &NodePath, visited: &mut HashSet<NodePath>) {
        let styles = scaffold.get_styles();
        let direction = flex_direction(styles);
        let gap = gap_px(styles);
        let (top, right, bottom, left) = padding_insets(styles);
        let content_rect = NSRect::new(
            NSPoint::new(available_rect.origin.x + left, available_rect.origin.y + top),
            NSSize::new((available_rect.size.width - left - right).max(0.0), (available_rect.size.height - top - bottom).max(0.0)),
        );

        let entries: Vec<((std::any::TypeId, usize), &Scaffold<'ctx>)> =
            scaffold.get_slots().iter().filter(|(_, child)| child.is_enabled()).map(|(key, child)| (*key, child)).collect();

        if entries.is_empty() {
            return;
        }

        struct Child<'a, 'ctx> {
            path: NodePath,
            scaffold: &'a Scaffold<'ctx>,
            kind: NodeKind,
            native: NativeNode,
            flex: Option<f64>,
            main_size: f64,
            cross_size: Option<f64>,
        }

        let mut children = Vec::with_capacity(entries.len());

        for (key, child) in entries {
            let path = base_path.child(key);
            visited.insert(path.clone());

            let kind = classify(child);
            let native = self.spawn_or_patch(&path, &kind, parent_view);

            let child_styles = child.get_styles();
            let flex = flex_weight(child_styles);
            let explicit_main = explicit_main_px(child_styles, direction);
            let cross_size = explicit_cross_px(child_styles, direction);

            let main_size = if flex.is_some() {
                0.0 // resolved below, once every child's fixed/intrinsic size is known
            } else if let Some(px) = explicit_main {
                px
            } else {
                self.intrinsic_main_size(&native, direction)
            };

            children.push(Child { path, scaffold: child, kind, native, flex, main_size, cross_size });
        }

        let total_gap = gap * (children.len().saturating_sub(1)) as f64;
        let main_axis_extent = if direction == FlexDirection::Row { content_rect.size.width } else { content_rect.size.height };
        let fixed_total: f64 = children.iter().filter(|child| child.flex.is_none()).map(|child| child.main_size).sum();
        let flex_total: f64 = children.iter().filter_map(|child| child.flex).sum();
        let remaining = (main_axis_extent - fixed_total - total_gap).max(0.0);

        let mut cursor = if direction == FlexDirection::Row { content_rect.origin.x } else { content_rect.origin.y };

        for Child { path, scaffold: child_scaffold, kind, native, flex, mut main_size, cross_size } in children {
            if let Some(weight) = flex {
                main_size = if flex_total > 0.0 { remaining * (weight / flex_total) } else { 0.0 };
            }

            let cross_extent = if direction == FlexDirection::Row { content_rect.size.height } else { content_rect.size.width };
            let cross_origin = if direction == FlexDirection::Row { content_rect.origin.y } else { content_rect.origin.x };
            let resolved_cross = cross_size.unwrap_or(cross_extent);

            let frame = if direction == FlexDirection::Row {
                NSRect::new(NSPoint::new(cursor, cross_origin), NSSize::new(main_size, resolved_cross))
            } else {
                NSRect::new(NSPoint::new(cross_origin, cursor), NSSize::new(resolved_cross, main_size))
            };

            native.as_view().setFrame(frame);

            if matches!(kind, NodeKind::Container | NodeKind::TabRow { .. }) {
                // A child's own children are positioned relative to *its* bounds, not the outer
                // surface's — same convention every `NSView` subview tree already uses.
                let child_view: Retained<NSView> = match &native {
                    NativeNode::Container(view) => Retained::clone(view),
                    NativeNode::Row(view) => Retained::into_super(Retained::clone(view)),
                    _ => unreachable!("classify() only returns NodeKind::Container/TabRow for these native shapes"),
                };
                let local_bounds = NSRect::new(NSPoint::new(0.0, 0.0), frame.size);
                self.layout_children(child_scaffold, &child_view, local_bounds, &path, visited);
            }

            cursor += main_size + gap;

            self.nodes.insert(path, native);
        }
    }

    fn intrinsic_main_size(&self, native: &NativeNode, direction: FlexDirection) -> f64 {
        match native {
            NativeNode::Button { view, .. } => {
                view.sizeToFit();
                let size = view.frame().size;
                if direction == FlexDirection::Row { size.width } else { size.height }
            }
            _ => 0.0,
        }
    }

    fn spawn_or_patch(&mut self, path: &NodePath, kind: &NodeKind, parent_view: &NSView) -> NativeNode {
        let existing = self.nodes.remove(path);

        let same_kind = matches!(
            (&existing, kind),
            (Some(NativeNode::Container(_)), NodeKind::Container)
                | (Some(NativeNode::Button { .. }), NodeKind::Button { .. })
                | (Some(NativeNode::TextField { .. }), NodeKind::TextField { .. })
                | (Some(NativeNode::Label(_)), NodeKind::Label(_))
                | (Some(NativeNode::Image(_)), NodeKind::Favicon { .. })
                | (Some(NativeNode::Row(_)), NodeKind::TabRow { .. })
        );

        let native = if same_kind {
            existing.unwrap()
        } else {
            if let Some(stale) = existing {
                stale.as_view().removeFromSuperview();
            }
            let fresh = self.spawn(path, kind);
            parent_view.addSubview(fresh.as_view());
            fresh
        };

        self.patch(&native, kind);
        native
    }

    fn spawn(&mut self, path: &NodePath, kind: &NodeKind) -> NativeNode {
        let mtm = self.mtm;
        let zero_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));

        match kind {
            NodeKind::Container => NativeNode::Container(Retained::into_super(FlippedView::new(mtm, zero_frame))),

            NodeKind::Button { .. } => {
                let view = NSButton::initWithFrame(NSButton::alloc(mtm), zero_frame);
                if self.theme.is_some() {
                    // A themed toolbar reads as flat toolbar glyphs (back/forward/refresh/new-tab)
                    // rather than full macOS push buttons — `NSBezelStyle::Push`'s beveled chrome
                    // is the "clunky, inconsistent" look this replaces; an unthemed surface (no
                    // `Theme` set) keeps the original default untouched.
                    view.setBordered(false);
                    view.setContentTintColor(self.theme.map(|theme| rgb_color(theme.text)).as_deref());
                    // Default system-font size (13pt) reads as small/cramped for a lone glyph
                    // (‹, ›, ↻, ☰, ×) once the button chrome around it is gone — a themed surface
                    // bumps it for legibility, same reasoning as the wider `Size::width` values in
                    // `escher-chalk`'s `toolbar` and this crate's `tabs` compositions.
                    if let Some(theme) = self.theme {
                        view.setFont(Some(&NSFont::systemFontOfSize(theme.ui_text_size)));
                    }
                } else {
                    view.setBezelStyle(NSBezelStyle::Push);
                }

                let outbox = self.outbox.clone();
                let path = path.clone();
                let wake = self.wake.clone();
                let target = ActionTarget::new(mtm, move || {
                    outbox.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push((path.clone(), NativeEvent::Activated));
                    if let Some(wake) = &wake {
                        wake();
                    }
                });
                unsafe {
                    view.setTarget(Some(&target));
                    view.setAction(Some(objc2::sel!(fire:)));
                }

                // Only themed buttons get a hover tint — an unthemed surface has no `accent`
                // color to tint toward, and keeps its default system push-button hover behavior
                // (which AppKit already provides for free) untouched.
                let hover = self.theme.map(|theme| {
                    let view_for_tint = view.clone();
                    crate::hover::HoverTarget::attach(mtm, &view, move |hovering| {
                        let color = if hovering { theme.accent } else { theme.text };
                        view_for_tint.setContentTintColor(Some(&rgb_color(color)));
                    })
                });

                NativeNode::Button { view, _target: target, _hover: hover }
            }

            NodeKind::TextField { .. } => {
                let view = NSTextField::initWithFrame(NSTextField::alloc(mtm), zero_frame);
                view.setBezeled(true);
                view.setEditable(true);
                view.setDrawsBackground(true);
                if let Some(theme) = self.theme {
                    view.setTextColor(Some(&rgb_color(theme.text)));
                    view.setBackgroundColor(Some(&rgb_color(theme.surface)));
                    view.setFont(Some(&NSFont::systemFontOfSize(theme.body_text_size)));
                }

                let outbox = self.outbox.clone();
                let path = path.clone();
                let wake = self.wake.clone();
                let field_for_read = view.clone();
                let target = ActionTarget::new(mtm, move || {
                    let text = field_for_read.stringValue().to_string();
                    outbox.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push((path.clone(), NativeEvent::Submitted(text)));
                    if let Some(wake) = &wake {
                        wake();
                    }
                });
                unsafe {
                    view.setTarget(Some(&target));
                    view.setAction(Some(objc2::sel!(fire:)));
                }

                NativeNode::TextField { view, _target: target }
            }

            NodeKind::Label(_) => {
                let view = NSTextField::initWithFrame(NSTextField::alloc(mtm), zero_frame);
                view.setBezeled(false);
                view.setEditable(false);
                view.setSelectable(false);
                view.setDrawsBackground(false);
                // A label's box is whatever its slot's layout gives it (e.g. a tab title's `Flex`
                // width, which shrinks as the row gets crowded) — without this, text that doesn't
                // fit wraps to a second line inside a fixed-height row and gets clipped top/bottom,
                // which is the main way this surface's text has been rendering badly. Truncating
                // to one line with a trailing ellipsis is the standard native behavior instead.
                view.setLineBreakMode(objc2_app_kit::NSLineBreakMode::ByTruncatingTail);
                view.setUsesSingleLineMode(true);
                if let Some(theme) = self.theme {
                    view.setTextColor(Some(&rgb_color(theme.text)));
                    view.setFont(Some(&NSFont::systemFontOfSize(theme.body_text_size)));
                }

                NativeNode::Label(view)
            }

            NodeKind::Favicon { .. } => {
                let view = NSImageView::initWithFrame(NSImageView::alloc(mtm), zero_frame);
                NativeNode::Image(view)
            }

            NodeKind::TabRow { .. } => {
                let outbox = self.outbox.clone();
                let path = path.clone();
                let wake = self.wake.clone();
                let view = crate::views::TabRowView::new(
                    mtm,
                    zero_frame,
                    move |delta| {
                        outbox.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push((path.clone(), NativeEvent::RowReleased(delta)));
                        if let Some(wake) = &wake {
                            wake();
                        }
                    },
                    self.theme.map(|theme| theme.accent),
                );

                NativeNode::Row(view)
            }
        }
    }

    fn patch(&mut self, native: &NativeNode, kind: &NodeKind) {
        match (native, kind) {
            (NativeNode::Button { view, .. }, NodeKind::Button { label, disabled }) => {
                view.setTitle(&NSString::from_str(label));
                view.setEnabled(!disabled);
            }
            (NativeNode::TextField { view, .. }, NodeKind::TextField { value, placeholder }) => {
                // Skip touching `stringValue` at all while the user has this field open for
                // editing (`currentEditor()` is only `Some` for the first-responder text field) —
                // every draw call re-patches from the *last committed* `value`, which is stale by
                // definition while the user is mid-keystroke, so comparing against it and patching
                // on a mismatch (the previous approach) stomped every character the instant it was
                // typed. Once editing ends (blur or Enter-commit), `value` catches up via the
                // `Submitted`/`Navigate` round-trip and this resumes syncing normally.
                if view.currentEditor().is_none() && view.stringValue().to_string() != *value {
                    view.setStringValue(&NSString::from_str(value));
                }
                view.setPlaceholderString(placeholder.as_deref().map(NSString::from_str).as_deref());
            }
            (NativeNode::Label(view), NodeKind::Label(text)) => {
                view.setStringValue(&NSString::from_str(text));
            }
            (NativeNode::Image(view), NodeKind::Favicon { host }) => {
                view.setImage(self.favicons.get(host).as_deref());
            }
            (NativeNode::Container(_), NodeKind::Container) => {}
            (NativeNode::Row(view), NodeKind::TabRow { selected }) => view.set_selected(*selected),
            _ => unreachable!("spawn_or_patch only calls patch with a native/kind pair of matching shape"),
        }
    }
}
