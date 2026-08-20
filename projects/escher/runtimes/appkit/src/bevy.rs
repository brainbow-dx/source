//! Optional Bevy-engine glue for this crate's toolbar/tab-strip/global-shortcuts surfaces — gated
//! behind this crate's own `bevy` feature (see `Cargo.toml`), not `escher-bevy`: a Bevy-integration
//! crate meant to stay agnostic of which native UI backend renders a given surface (the same way
//! it doesn't know `escher-webview` happens to be `WKWebView`-backed today) has no business
//! depending on one specific backend crate directly. This module is the mirror image of that:
//! every native AppKit call it makes (`AppKitSurface`, `GlobalShortcuts`, `objc2::MainThreadMarker`)
//! stays inside it — nothing this module exports (`WantsToolbar`, `WantsTabStrip`, `ToolbarState`,
//! `TabStripState`, `ToolbarEvent`, `TabStripEvent`, `ToolbarPlugin`) is AppKit-specific, so a
//! consumer (an app, or eventually `escher-bevy` itself once more than one backend exists to choose
//! between) never needs to name this crate's own internals just to use it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::app::{App, Plugin, Update};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::Message;
use bevy::ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy::ecs::system::{Commands, NonSendMut, Res};
use bevy::prelude::{MessageWriter, Query, Resource, With};
use bevy::window::RawHandleWrapper;

use crate::surface::AppKitSurface;

pub use crate::TOOLBAR_HEIGHT;
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
}

/// What the tab strip shows — the app owns `tabs`/`active`/`collapsed`; `width` is the strip's
/// full (uncollapsed) width, set once and left alone unless the app wants to resize it. Call
/// `effective_width()` for what to actually reserve elsewhere (a webview's left inset, say) —
/// `0.0` while `collapsed`, `width` otherwise.
#[derive(Resource, Clone)]
pub struct TabStripState {
    pub tabs: Vec<TabInfo>,
    pub active: Option<u64>,
    pub collapsed: bool,
    pub width: f64,
}

impl TabStripState {
    pub fn effective_width(&self) -> f64 {
        if self.collapsed { 0.0 } else { self.width }
    }
}

/// The theme newly-attached toolbar/tab-strip surfaces are created with — set this (from a
/// styleguide, typically) before spawning `WantsToolbar`/`WantsTabStrip` entities if you want
/// anything other than plain system-default AppKit chrome; the default here matches the look
/// this crate had before theming existed (nothing painted, system colors throughout).
#[derive(Resource, Default, Clone, Copy)]
pub struct ThemeState(pub Option<ToolbarTheme>);

impl Default for TabStripState {
    /// `220.0` is a reasonable starting sidebar width, not a load-bearing constant — override
    /// `width` after inserting if a consumer wants something else.
    fn default() -> Self {
        TabStripState { tabs: Vec::new(), active: None, collapsed: false, width: 220.0 }
    }
}

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
}

/// Fired by tab-strip interactions — select/close/reorder an existing row, or open a new one.
#[derive(Message, Debug, Clone)]
pub enum TabStripEvent {
    Select(u64),
    Close(u64),
    Reorder(u64, i32),
    New,
}

/// Attaches a toolbar (and, on windows that also asked for one, a tab strip) once a window's
/// native handle is ready, redraws both every tick, and translates native interactions plus global
/// keyboard/mouse shortcuts into `ToolbarEvent`/`TabStripEvent`.
pub struct ToolbarPlugin;

/// Every system this plugin schedules runs inside this set, in a fixed internal order — a
/// consumer that needs to run before/after all of them (writing `ToolbarState`/`TabStripState`
/// beforehand, reading `ToolbarEvent`/`TabStripEvent` afterward, say) orders against this set
/// rather than needing to name any of this plugin's own system functions, none of which are
/// public.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolbarSystems;

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToolbarEvent>();
        app.add_message::<TabStripEvent>();
        app.insert_non_send_resource(ToolbarSurfaces::default());
        app.insert_non_send_resource(TabStripSurfaces::default());
        app.insert_non_send_resource(PendingEvents::default());
        app.insert_non_send_resource(InstalledShortcuts::default());
        app.insert_resource(ToolbarState::default());
        app.init_resource::<ThemeState>();
        app.add_systems(
            Update,
            (
                attach_pending_toolbars,
                attach_pending_tab_strips,
                install_global_shortcuts,
                redraw_toolbar,
                redraw_tab_strip,
                dispatch_pending_events,
            )
                .chain()
                .in_set(ToolbarSystems),
        );
    }
}

/// Every currently-attached toolbar/tab-strip `AppKitSurface`, keyed by the window entity it's
/// attached to. `NonSend` since `AppKitSurface` wraps native AppKit objects.
#[derive(Default)]
struct ToolbarSurfaces(HashMap<Entity, AppKitSurface>);
#[derive(Default)]
struct TabStripSurfaces(HashMap<Entity, AppKitSurface>);

/// Builds the callback `AppKitSurface::set_wake_callback` needs from Bevy's own event-loop
/// handle — see `Theme`'s sibling field `wake` in `surface.rs` for why this matters: without it, a
/// click on one of these native views goes unnoticed by Bevy's `Update` schedule until
/// `WinitSettings::desktop_app()`'s idle-fallback timer fires, which reads as the UI being
/// unresponsive for up to several seconds after a real click.
fn wake_callback(event_loop_proxy: &Res<bevy::winit::EventLoopProxyWrapper>) -> std::sync::Arc<dyn Fn() + Send + Sync + 'static> {
    // Fully-qualified, with the target type spelled out: ordinary `.clone()` method-call sugar
    // would resolve to `&Res<..>`'s own blanket reference `Clone` (every `&T` is trivially
    // `Clone`) before ever reaching the real `EventLoopProxy::clone` two `Deref` hops down (through
    // `Res` then `EventLoopProxyWrapper`), silently cloning the *reference* — tied to this
    // system's borrow — instead of the owned, `'static`-capable proxy underneath.
    let proxy: bevy::winit::EventLoopProxy<bevy::winit::WinitUserEvent> = Clone::clone(event_loop_proxy);
    std::sync::Arc::new(move || {
        let _ = proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
    })
}

fn attach_pending_toolbars(
    mut commands: Commands,
    mut surfaces: NonSendMut<ToolbarSurfaces>,
    theme: Res<ThemeState>,
    event_loop_proxy: Res<bevy::winit::EventLoopProxyWrapper>,
    pending: Query<(Entity, &RawHandleWrapper), With<WantsToolbar>>,
) {
    for (entity, raw_handle) in &pending {
        match AppKitSurface::attach(raw_handle.get_window_handle(), crate::TOOLBAR_HEIGHT) {
            Ok(mut surface) => {
                tracing::info!("Attached toolbar to {entity:?}");
                if let Some(theme) = theme.0 {
                    surface.set_theme(theme);
                }
                surface.set_wake_callback(wake_callback(&event_loop_proxy));
                surfaces.0.insert(entity, surface);
            }
            Err(error) => tracing::warn!("Failed to attach toolbar: {error}"),
        }

        commands.entity(entity).remove::<WantsToolbar>();
    }
}

fn attach_pending_tab_strips(
    mut commands: Commands,
    mut surfaces: NonSendMut<TabStripSurfaces>,
    state: Res<TabStripState>,
    theme: Res<ThemeState>,
    event_loop_proxy: Res<bevy::winit::EventLoopProxyWrapper>,
    pending: Query<(Entity, &RawHandleWrapper), With<WantsTabStrip>>,
) {
    for (entity, raw_handle) in &pending {
        match AppKitSurface::attach_sidebar(raw_handle.get_window_handle(), state.effective_width(), crate::TOOLBAR_HEIGHT) {
            Ok(mut surface) => {
                tracing::info!("Attached tab strip to {entity:?}");
                if let Some(theme) = theme.0 {
                    surface.set_theme(theme);
                }
                surface.set_wake_callback(wake_callback(&event_loop_proxy));
                surfaces.0.insert(entity, surface);
            }
            Err(error) => tracing::warn!("Failed to attach tab strip: {error}"),
        }

        commands.entity(entity).remove::<WantsTabStrip>();
    }
}

/// What a native callback (a button click, a tab row release, a global shortcut firing) queues up
/// — `AppKitSurface::draw`'s own closures can't hold a `MessageWriter` (a normal system param,
/// borrowed only for the duration of one system call, not `'static`), so they push here instead;
/// `dispatch_pending_events` drains it into real `Message` writes right after. Same outbox shape
/// `AppKitSurface` itself uses internally, one level up — see its own module doc comment.
#[derive(Default, Clone)]
struct PendingEvents {
    toolbar: Arc<Mutex<Vec<ToolbarEvent>>>,
    tabs: Arc<Mutex<Vec<TabStripEvent>>>,
}

fn redraw_toolbar(mut surfaces: NonSendMut<ToolbarSurfaces>, state: Res<ToolbarState>, pending: NonSendMut<PendingEvents>) {
    let address = state.address.clone();
    let loading = state.loading;

    for surface in surfaces.0.values_mut() {
        let address = address.clone();
        let toggle_queue = pending.toolbar.clone();
        let back_queue = pending.toolbar.clone();
        let forward_queue = pending.toolbar.clone();
        let refresh_queue = pending.toolbar.clone();
        let load_queue = pending.toolbar.clone();

        surface.draw(move |root| {
            escher_chalk::toolbar::toolbar(
                root,
                &address,
                loading,
                move || toggle_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::ToggleSidebar),
                move || back_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Back),
                move || forward_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Forward),
                move || refresh_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Refresh),
                move |text| load_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Navigate(text)),
            )
        });
    }
}

fn redraw_tab_strip(mut surfaces: NonSendMut<TabStripSurfaces>, state: Res<TabStripState>, pending: NonSendMut<PendingEvents>) {
    let width = state.effective_width();
    let active = state.active;

    for surface in surfaces.0.values_mut() {
        surface.set_width(width);

        let tabs = state.tabs.clone();
        let select_queue = pending.tabs.clone();
        let close_queue = pending.tabs.clone();
        let reorder_queue = pending.tabs.clone();
        let new_tab_queue = pending.tabs.clone();

        surface.draw(move |root| {
            crate::tabs::tab_strip(
                root,
                &tabs,
                active,
                move |id| select_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(TabStripEvent::Select(id)),
                move |id| close_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(TabStripEvent::Close(id)),
                move |id, positions| reorder_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(TabStripEvent::Reorder(id, positions)),
                move || new_tab_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(TabStripEvent::New),
            )
        });
    }
}

fn dispatch_pending_events(pending: NonSendMut<PendingEvents>, mut toolbar_events: MessageWriter<ToolbarEvent>, mut tab_events: MessageWriter<TabStripEvent>) {
    for event in std::mem::take(&mut *pending.toolbar.lock().unwrap_or_else(|poisoned| poisoned.into_inner())) {
        toolbar_events.write(event);
    }
    for event in std::mem::take(&mut *pending.tabs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())) {
        tab_events.write(event);
    }
}

/// Holds the global mouse/keyboard back-forward-refresh monitor once it exists — `NonSend`
/// (`GlobalShortcuts` wraps a native `NSEvent` monitor token), `Option` since there's nothing to
/// install until at least one toolbar-bearing window's native handle is ready.
#[derive(Default)]
struct InstalledShortcuts(Option<crate::shortcuts::GlobalShortcuts>);

/// Installs the global back/forward/refresh monitor once, the moment any toolbar window's native
/// handle is ready — mouse side-buttons and Cmd+[/Cmd+]/Cmd+R then work anywhere in the app, not
/// just when the toolbar itself has focus. Routes into the same `PendingEvents` queue a real
/// button click would.
fn install_global_shortcuts(mut installed: NonSendMut<InstalledShortcuts>, pending: NonSendMut<PendingEvents>, window_query: Query<&RawHandleWrapper, With<WantsToolbar>>) {
    if installed.0.is_some() {
        return;
    }

    // At least one window with `WantsToolbar` must exist for shortcuts to be worth installing —
    // but `attach_pending_toolbars` removes the marker once attached, so this can't just query
    // `With<WantsToolbar>` after that happens. Installing the moment *any* window handle exists
    // (regardless of marker) is simpler and correct for a single-toolbar-window app; a consumer
    // wanting per-window shortcuts would need a different mechanism entirely, not a tweak to this.
    let _ = window_query;

    let Some(mtm) = objc2::MainThreadMarker::new() else { return };

    let back_queue = pending.toolbar.clone();
    let forward_queue = pending.toolbar.clone();
    let refresh_queue = pending.toolbar.clone();

    installed.0 = Some(crate::shortcuts::GlobalShortcuts::install(
        mtm,
        move || back_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Back),
        move || forward_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Forward),
        move || refresh_queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(ToolbarEvent::Refresh),
    ));

    tracing::info!("Installed global back/forward/refresh shortcuts");
}
