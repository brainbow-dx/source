//! Minimal example proving out the pieces ported from the pre-refactor `slate` prototype:
//! `EscherBevyPlugin` (window bootstrap), `ReticlePlugin` (cursor-following gizmo), and, with
//! the `terminal` feature, `TerminalPlugin` (a `ratatui` debug console drawn to the real OS
//! terminal alongside the game window). Doesn't touch `escher-core` — see `src/legacy/README.md`
//! for why the actual Escher-render bridge (`provider.rs`) isn't wired up here yet.

use bevy::prelude::*;

use escher_bevy::EscherBevyConfig;
use escher_bevy::EscherBevyPlugin;

fn main() {
    App::new()
        .add_plugins(EscherBevyPlugin::new(
            EscherBevyConfig::default().with_clear_color(Color::hsla(220.0, 0.11, 0.11, 1.0)),
        ))
        .add_systems(Startup, spawn_camera)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
