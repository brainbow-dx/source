//! The top-level Escher Bevy plugin. Ported from a pre-refactor `slate` prototype — trimmed
//! down to the parts that are actually cross-platform. The original also wired up a custom
//! window manager and a WebView overlay, but both of those (`window.rs`/`webview.rs`) turned
//! out to be Windows-only (hard-locked to the `webview2` COM API, no macOS backend was ever
//! written) — they're kept as reference in `src/legacy/` rather than pulled in here. Likewise
//! `provider.rs` (the actual Escher-`Scaffold`-to-Bevy-UI bridge) targets the pre-refactor core
//! API and needs a real rewrite against the current `escher-core`, so it's parked in
//! `src/legacy/` too rather than wired in half-working.

use bevy::DefaultPlugins;
use bevy::app::App;
use bevy::app::Plugin;
use bevy::app::PluginGroup;
use bevy::asset::AssetPlugin;
use bevy::camera::ClearColor;
use bevy::window::Window;
use bevy::window::WindowLevel;
use bevy::window::WindowMode;
use bevy::winit::WinitSettings;

use crate::config::EscherBevyConfig;

#[derive(Default)]
pub struct EscherBevyPlugin {
    config: EscherBevyConfig,
}

impl EscherBevyPlugin {
    pub fn new(config: EscherBevyConfig) -> Self {
        EscherBevyPlugin { config }
    }
}

impl Plugin for EscherBevyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings::desktop_app());
        app.insert_resource(ClearColor(self.config.clear_color));

        if self.config.bevy_defaults {
            tracing::debug!("Enabling Bevy's default plugins ..");
            self.apply_bevy_defaults(app);
        }

        #[cfg(feature = "terminal")]
        app.add_plugins(crate::terminal::TerminalPlugin::new());

        app.add_plugins(crate::reticle::ReticlePlugin::new());
    }
}

impl EscherBevyPlugin {
    fn apply_bevy_defaults(&self, app: &mut App) {
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: self.config.asset_dir.clone(),
                    watch_for_changes_override: Some(true),
                    ..Default::default()
                })
                .set(bevy::window::WindowPlugin {
                    primary_window: self.config.spawn_primary_window.then(|| {
                        create_window(
                            &self.config.window_title,
                            self.config.window_width,
                            self.config.window_height,
                            self.config.window_visible,
                            WindowLevel::Normal,
                        )
                    }),
                    exit_condition: self.config.exit_condition.clone(),
                    close_when_requested: self.config.close_when_requested,
                    ..Default::default()
                })
                // Prefer Escher's own logging setup over Bevy's default.
                .disable::<bevy::log::LogPlugin>(),
        );
    }
}

/// Builds the primary window from `EscherBevyConfig`'s own title/size/visibility fields — pulled
/// out to a plain function, not a method on `EscherBevyPlugin`, so nothing about window shape is
/// hidden inside the plugin: the config a caller already builds is the one place all of this
/// lives.
pub fn create_window(title: &str, width: f32, height: f32, visible: bool, window_level: WindowLevel) -> Window {
    Window {
        title: String::from(title),
        mode: WindowMode::Windowed,
        resolution: bevy::window::WindowResolution::new(width as u32, height as u32),
        transparent: false,
        decorations: true,
        resizable: true,
        visible,
        position: bevy::window::WindowPosition::Centered(bevy::window::MonitorSelection::Current),
        composite_alpha_mode: bevy::window::CompositeAlphaMode::Opaque,
        window_level,
        ..Default::default()
    }
}
