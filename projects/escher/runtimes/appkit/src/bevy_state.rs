//! The toolbar/tab-strip data model itself — what to show, what happened — kept apart from
//! `bevy.rs`'s own ECS scheduling glue (the `Plugin`, its systems, the native-callback wiring).
//! Per the user directly: none of `ToolbarState`/`TabStripState`/`ToolbarEvent`/`TabStripEvent`
//! actually *need* Bevy specifically — they're plain data and event enums a consumer could equally
//! well hand to some other engine's ECS, or to no ECS at all — so burying them inside a file whose
//! own doc comment calls itself "Bevy-engine glue" made them harder to find than they should be,
//! and harder for a future non-Bevy integration to reuse without wading through 300+ lines of
//! system-scheduling code to find the actual schema. Still gated behind this crate's own `bevy`
//! feature today, same as `bevy.rs` — `#[derive(Resource)]`/`#[derive(Message)]` are genuinely
//! Bevy-specific derives, so this doesn't (yet) remove the dependency entirely, only the
//! conflation of "what" with "how it's wired into one particular engine."

use bevy::ecs::component::Component;
use bevy::ecs::message::Message;
use bevy::prelude::Resource;

pub use crate::TOOLBAR_HEIGHT;
pub use crate::surface::RESIZE_HANDLE_WIDTH;
pub use crate::surface::Theme as ToolbarTheme;
pub use escher_chalk::tabs::TabInfo;

/// Marks a window entity that should get a toolbar attached once its native handle is ready —
/// removed once attached (successfully or not) so it's never retried.
#[derive(Component)]
pub struct WantsToolbar;

/// Marks a window entity that should get a tab strip attached alongside its toolbar, once its
/// native handle is ready — same lifetime/removal contract as `WantsToolbar`. Reads
/// `TabStripState::width` (must already be inserted) for the sidebar's starting width.
#[derive(Component)]
pub struct WantsTabStrip;

/// What the toolbar's address field shows — the app writes this every tick from its own notion of
/// "current page" (e.g. the active tab's URL); the plugin only ever reads it.
#[derive(Resource, Default, Clone)]
pub struct ToolbarState {
    pub address: String,
    /// Drives the refresh button's glyph swap — see `escher_chalk::toolbar::toolbar`'s own doc
    /// comment on its `loading` parameter, which this is passed straight through to. The app sets
    /// this from whatever it actually knows is loading (a webview's own `is_loading()`, typically)
    /// every tick, same as `address`.
    pub loading: bool,
    /// Whether the pin button reads as toggled on — unlike `address`/`loading`, this isn't
    /// resynced from anything else every tick; `ToolbarEvent::TogglePinned`'s own handler flips
    /// this directly, and it's also what that same handler reads to decide the host window's
    /// real `WindowLevel`. The app still owns setting its *initial* value (from `--always-on-top`/
    /// `.anvil.toml`, in `apps/anvil`'s case) before the first draw.
    pub pinned: bool,
}

/// What the tab strip shows — the app owns `tabs`/`active`; `width` is the strip's current width,
/// set once and left alone unless the app (or a live drag) wants to resize it. Call
/// `effective_width()` for what to actually reserve elsewhere (a webview's left inset, say).
///
/// No separate "collapsed/hidden" flag — hiding the sidebar outright is the wrong behavior for
/// the toggle button; it collapses to
/// `ICON_RAIL_WIDTH` (the same favicon-only layout dragging below `ICON_ONLY_WIDTH` already snaps
/// into) instead. `width` alone captures the whole picture, including for persistence — see
/// `expanded_width`'s own doc comment for the one piece of state a plain `width` can't.
#[derive(Resource, Clone)]
pub struct TabStripState {
    pub tabs: Vec<TabInfo>,
    pub active: Option<u64>,
    pub width: f64,
    /// The width to restore to when the toggle button expands the sidebar back out — updated
    /// automatically whenever the user drags to (or the app otherwise sets) a width at or above
    /// `ICON_ONLY_WIDTH`, so "collapse, then expand" always lands back where it started rather
    /// than some fixed default. Needs its own field because `width` itself gets overwritten with
    /// `ICON_RAIL_WIDTH` while collapsed — this is the one thing a plain `width` can't remember on
    /// its own.
    pub expanded_width: f64,
}

/// Below this, dragging the resize handle snaps the sidebar into `tab_strip`'s icon-only layout
/// (see `TabStripState::icon_only`) instead of continuing to narrow the labeled one — the same
/// "drag it thin enough and it becomes an icon rail" convention VS Code's own sidebar uses, rather
/// than a separate toggle needing its own cycle-through-three-states decision. Purely a rendering
/// concern (which layout `tab_strip` picks), unlike `MIN_WIDTH`/`MAX_WIDTH` — those also bound the
/// *native* drag directly (see `crate::surface`'s own doc comment on why), so they live there
/// instead, re-exported here for the same single-import convenience `RESIZE_HANDLE_WIDTH` gets.
pub const ICON_ONLY_WIDTH: f64 = 72.0;
/// What the toggle button collapses the sidebar *to* — comfortably inside `icon_only`'s own
/// territory (below `ICON_ONLY_WIDTH`), and clear of `MIN_WIDTH`, so it always reads as a
/// deliberate icon rail rather than the absolute minimum a drag could reach.
pub const ICON_RAIL_WIDTH: f64 = 56.0;
pub use crate::surface::{MAX_WIDTH, MIN_WIDTH};

impl TabStripState {
    pub fn effective_width(&self) -> f64 {
        self.width
    }

    /// Whether `width` is currently thin enough that `crate::tabs::tab_strip` should render just
    /// favicons — a live, derived read of `width` rather than a separately stored flag, so there's
    /// exactly one source of truth and dragging back out above `ICON_ONLY_WIDTH` always instantly
    /// restores full rows.
    pub fn icon_only(&self) -> bool {
        self.width < ICON_ONLY_WIDTH
    }

    /// The toggle button's own behavior: collapse to `ICON_RAIL_WIDTH` (remembering the current
    /// width to restore to) if not already collapsed, or restore `expanded_width` if it is.
    pub fn toggle_collapsed(&mut self) {
        if self.icon_only() {
            self.width = self.expanded_width;
        } else {
            self.expanded_width = self.width;
            self.width = ICON_RAIL_WIDTH;
        }
    }
}

impl Default for TabStripState {
    /// `220.0` is a reasonable starting sidebar width, not a load-bearing constant — override
    /// `width` after inserting if a consumer wants something else.
    fn default() -> Self {
        TabStripState { tabs: Vec::new(), active: None, width: 220.0, expanded_width: 220.0 }
    }
}

/// The theme newly-attached toolbar/tab-strip surfaces are created with — set this (from a
/// styleguide, typically) before spawning `WantsToolbar`/`WantsTabStrip` entities if you want
/// anything other than plain system-default AppKit chrome; the default here matches the look
/// this crate had before theming existed (nothing painted, system colors throughout).
#[derive(Resource, Default, Clone, Copy)]
pub struct ThemeState(pub Option<ToolbarTheme>);

/// Fired when the toolbar's back/forward/refresh/sidebar-toggle buttons are clicked, its address
/// field is submitted, *or* the global mouse/keyboard shortcuts fire (see
/// `crate::shortcuts::GlobalShortcuts` — both routes land here identically, since a consumer
/// reacting to "go back" shouldn't care which one triggered it).
#[derive(Message, Debug, Clone)]
pub enum ToolbarEvent {
    Back,
    Forward,
    Refresh,
    Navigate(String),
    ToggleSidebar,
    /// The toolbar's pin button — "always on top" is a real, per-window, live-toggleable choice,
    /// not a build-mode accident. The consumer's own handler
    /// both flips `ToolbarState::pinned` and applies the corresponding `WindowLevel` to the real
    /// host window; this crate has no window entity of its own to do that with.
    TogglePinned,
}

/// Fired by tab-strip interactions — select/close/reorder an existing row, open a new one, or drag
/// the sidebar's resize handle (`f64` is the horizontal delta in points since the last tick, not
/// an absolute width — the consumer already owns `TabStripState::width` and just adds this to it).
#[derive(Message, Debug, Clone)]
pub enum TabStripEvent {
    Select(u64),
    Close(u64),
    Reorder(u64, i32),
    New,
    Resize(f64),
}
