//! Minimal proof that `escher-webview` can attach a real native webview to a Bevy/winit window on
//! whatever platform this is built for — no toolbar, no tabs, no OS chrome, none of `escher-appkit`
//! (whose `bevy` module, unlike this crate, is `#[cfg(target_os = "macos")]`-only today; see its
//! own `lib.rs`). Deliberately this small: `apps/anvil`'s full browser experience can't compile on
//! a second platform yet (it imports `escher_appkit::bevy` unconditionally for its toolbar/tab
//! strip), so this is the smallest real thing that *can* — the actual new capability a Windows
//! port needs to prove out first, isolated from that much bigger, separate, not-yet-started
//! "port the native chrome too" effort.

use bevy::prelude::*;
use bevy::window::RawHandleWrapper;

use escher_bevy::EscherBevyConfig;
use escher_bevy::EscherBevyPlugin;

use escher_webview::WebView;

fn main() {
    App::new()
        .add_plugins(EscherBevyPlugin::new(
            EscherBevyConfig::default()
                .with_window_title("Escher WebView Smoke Test")
                .with_window_size(1000.0, 700.0)
                // Pre-existing, unrelated to this example: `TerminalPlugin`'s own systems write
                // to a `SceneCommand` message type it never registers itself (assuming whatever
                // app uses it — `apps/anvil` — registers it, which this example has no reason
                // to). Confirmed live: `examples/hello.rs` panics on startup the same way,
                // untouched by anything here. Anvil itself disables this same plugin for its own,
                // different reason (it has its own terminal UI); this example just needs the
                // window, not any terminal at all.
                .with_spawn_terminal_plugin(false),
        ))
        // Not `Send` (wraps a native view handle — `NSView`/`ICoreWebView2Controller` depending
        // on platform), so this is a non-send resource, not a plain `Resource` — the same reason
        // `apps/anvil` keeps its own webviews in a `NonSend` resource (`TabWebViews`).
        .insert_non_send_resource(KeepAlive(None))
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, attach_webview_once)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Runs every tick until it succeeds exactly once — the primary window's native handle isn't
/// available the very first frame (same reason `apps/anvil`'s own `attach_pending_tab_webviews`
/// gives a fresh tab one full frame before attaching), so this just keeps trying instead of
/// assuming `Startup` is soon enough.
fn attach_webview_once(window_query: Query<&RawHandleWrapper>, mut keep_alive: NonSendMut<KeepAlive>, mut attached: Local<bool>) {
    if *attached {
        return;
    }
    let Ok(raw_handle) = window_query.single() else { return };

    match WebView::attach(
        raw_handle.get_window_handle(),
        "https://example.com",
        0.0,
        0.0,
        Some(escher_webview::DEFAULT_USER_AGENT),
        |_link_url| Vec::new(),
        None,
    ) {
        Ok(webview) => {
            tracing::info!("WebView attached");
            keep_alive.0 = Some(webview);
        }
        Err(error) => tracing::error!("WebView attach failed: {error}"),
    }

    *attached = true;
}

struct KeepAlive(#[allow(dead_code)] Option<WebView>);
