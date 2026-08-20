//! Thin Bevy-specific glue around `escher-webview` (which knows nothing about Bevy — it just
//! attaches a native webview given a raw window handle). This file's only job: attach a `WebView`
//! to whatever window a caller marks with `WantsWebView`, once that window's raw handle is ready.
//!
//! Deliberately knows nothing about `escher_os::chrome` or any other UI that might sit alongside
//! the webview — whether a given scene has a chrome bar (or a tab strip, or nothing at all) is a
//! decision for whatever Rust code scaffolds that specific scene (e.g. `apps/anvil`'s `main.rs`),
//! not something this reusable plugin should silently impose on every consumer. `WantsWebView::
//! top_inset` is this plugin's entire surface for that: a plain number of points to leave clear
//! at the top of the window, set by the caller however it sees fit.
//!
//! Also deliberately knows nothing about *how many* windows exist or how they're created — each
//! window is its own independent entity, and each `SceneCommand` opens a brand new one rather
//! than navigating a single shared webview in place. Navigating an *existing* window (chrome-bar
//! back/forward/address-bar submit) isn't this plugin's concern either: whatever owns that
//! window's own chrome bar already knows which window it's attached to, and can call `WebView::
//! load`/`go_back`/`go_forward` directly — see `apps/anvil`'s `poll_chrome_events`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::app::App;
use bevy::app::Plugin;
use bevy::app::Update;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::Message;
use bevy::ecs::system::Commands;
use bevy::ecs::system::NonSendMut;
use bevy::window::RawHandleWrapper;
use bevy::window::Window;
use bevy::prelude::Query;

use escher_webview::ContextMenuItem;
use escher_webview::CustomSchemeHandler;
use escher_webview::WebView;

/// Requests a brand new, independent scene window — fired from wherever a running app decides
/// one should open (a terminal command, e.g.). Each request always opens a *new* window; there's
/// no single shared webview to navigate in place. Consumed by whatever the app itself wires up to
/// react to it (spawning a `Window` + `WantsWebView`) — this plugin doesn't read this message
/// itself, since window shape (title, size, level, ...) is app policy, not this plugin's to decide.
/// Kept here as a shared, conventional shape rather than something every app redefines.
#[derive(Message, Debug, Clone)]
pub struct SceneCommand {
    pub url: String,
}

/// Marks a window entity that should get a `WebView` attached to it once its native handle is
/// ready — added by whatever spawns the window alongside the `Window` component itself. Removed
/// once attached (successfully or not) so it's never retried. The caller decides everything about
/// the window (title, size, level, visibility); this is this plugin's entire input.
#[derive(Component)]
pub struct WantsWebView {
    pub url: String,
    pub top_inset: f64,
    /// Points of clear space to leave on the left (e.g. for a tab strip). `0.0` fills the whole
    /// width below `top_inset`, same "no opinion" reasoning as `top_inset` itself.
    pub left_inset: f64,
    /// Forwarded straight to `escher_webview::WebView::attach`'s own `user_agent` parameter — see
    /// its doc comment (and `escher_webview::DEFAULT_USER_AGENT`'s) for what this is for. `None`
    /// keeps the platform webview's own default UA; this plugin has no opinion of its own on
    /// whether that's right for a given app.
    pub user_agent: Option<String>,
    /// Forwarded straight to `escher_webview::WebView::attach`'s own `on_link_context_menu` — see
    /// its doc comment. `None` means every link's context menu stays exactly WebKit's own default,
    /// same "no opinion unless asked" reasoning as `user_agent`. `Arc`, not a plain closure type,
    /// so this remains `Send + Sync` (required of any Bevy `Component`, which this is).
    pub on_link_context_menu: Option<Arc<dyn Fn(&str) -> Vec<ContextMenuItem> + Send + Sync>>,
    /// Forwarded straight to `escher_webview::WebView::attach`'s own `custom_scheme` — see its
    /// doc comment. `None` means no custom scheme is registered, same "no opinion unless asked"
    /// reasoning as `user_agent`/`on_link_context_menu`.
    pub custom_scheme: Option<CustomSchemeHandler>,
    /// Forwarded straight to `escher_webview::WebView::attach`'s own `initial_script`. Empty
    /// string (no script) is the common case, same "no opinion unless asked" reasoning as the
    /// fields above.
    pub initial_script: String,
}

pub struct WebViewPlugin;

impl Plugin for WebViewPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send_resource(WebViewHandles::default());
        app.add_message::<SceneCommand>();
        app.add_systems(Update, (attach_pending_webviews, clear_webviews_of_closed_windows));
    }
}

/// Every currently-attached `WebView`, keyed by the window entity it's attached to — `pub` so an
/// app with its own per-window UI (a chrome bar, say) can look one up to navigate it directly,
/// without this plugin needing to know that UI exists. `escher_webview::WebView` isn't `Send`/
/// `Sync` (it wraps a native AppKit object), so the whole map lives behind a `NonSend` resource
/// rather than being a per-entity `Component` — Bevy `Component`s need to be `Send`/`Sync`.
#[derive(Default)]
pub struct WebViewHandles(pub HashMap<Entity, WebView>);

fn attach_pending_webviews(
    mut commands: Commands,
    mut handles: NonSendMut<WebViewHandles>,
    pending: Query<(Entity, &RawHandleWrapper, &WantsWebView)>,
) {
    for (entity, raw_handle, wants) in &pending {
        let on_link_context_menu = wants.on_link_context_menu.clone();
        match WebView::attach(
            raw_handle.get_window_handle(),
            &wants.url,
            wants.top_inset,
            wants.left_inset,
            wants.user_agent.as_deref(),
            move |url| on_link_context_menu.as_ref().map(|f| f(url)).unwrap_or_default(),
            wants.custom_scheme.clone(),
            &wants.initial_script,
        ) {
            Ok(webview) => {
                handles.0.insert(entity, webview);
                tracing::info!("Opened scene: {}", wants.url);
            }
            Err(error) => tracing::warn!("Failed to attach webview for '{}': {error}", wants.url),
        }

        commands.entity(entity).remove::<WantsWebView>();
    }
}

/// A window's own close button despawns its entity (see `apps/anvil`'s window config — unlike the
/// single shared window this app used to have, each scene window closing independently is exactly
/// the point now, not something to intercept). Drops that window's `WebView` in step, rather than
/// leaving a dangling entry pointing at a native view whose window no longer exists.
fn clear_webviews_of_closed_windows(mut handles: NonSendMut<WebViewHandles>, window_query: Query<Entity, bevy::prelude::With<Window>>) {
    if handles.0.is_empty() {
        return;
    }

    let alive: std::collections::HashSet<Entity> = window_query.iter().collect();
    handles.0.retain(|entity, _| alive.contains(entity));
}
