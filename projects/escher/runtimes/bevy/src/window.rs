//! Builds a `bevy::window::Window` component — kept apart from `plugin.rs`'s own `Plugin::build`
//! wiring (registering `EscherBevyPlugin` with an `App`, adding `DefaultPlugins`) since window
//! *shape* isn't a plugin-registration concern; a caller spawning a second, later window (Anvil's
//! browser/scene windows, say) needs this without pulling in anything about how the primary window
//! or the plugin itself gets built.

use bevy::window::{Window, WindowLevel, WindowMode};

/// Builds a window from title/size/visibility — pulled out to a plain function, not a method on
/// `EscherBevyPlugin`, so nothing about window shape is hidden inside the plugin: the config a
/// caller already builds is the one place all of this lives.
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
