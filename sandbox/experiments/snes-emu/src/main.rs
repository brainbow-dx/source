//! Stub for the "mount a SNES emulator core in a Bevy/winit window" experiment — see
//! `projects/escher/spec/.agents/proposals/snes-emulator.md` for the full writeup (what a real
//! core would take, what already exists in the Rust ecosystem, and the much bigger "generate
//! real SNES machine code" stretch goal this is *not* attempting).
//!
//! Nothing emulator-related is wired in yet. This just opens a window and draws a placeholder
//! quad where a per-frame framebuffer texture would go, so starting this experiment for real
//! means replacing `spawn_placeholder_screen`/`update_placeholder_screen` below with an actual
//! core's `step()`/framebuffer output, not standing up a window from scratch.
//!
//! Deliberately standalone (its own `[workspace]`, not a member of Escher's), since this lives in
//! `sandbox/experiments/` specifically to stay decoupled from the real workspace while it's still
//! speculative. `bevy_audio`/`bevy_gilrs` (proven this session in `runtimes/bevy/examples/mario`
//! for exactly "play a stream of PCM samples"/"read gamepad input") are the natural fit for the
//! real core's APU output and controller input once one exists — not pulled in here since there's
//! nothing yet to feed them.

use bevy::color::Color;
use bevy::prelude::*;

/// Stand-in for "the emulator's current framebuffer, as a texture." A real core would replace
/// this resource with one holding a `Handle<Image>` updated once per emulated frame from the
/// core's actual pixel output (256x224, SNES's base resolution), not a static placeholder color.
#[derive(Resource)]
struct PlaceholderScreen {
    entity: Entity,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "SNES emulator experiment (stub, not started)".to_string(), ..default() }),
            ..default()
        }))
        .add_systems(Startup, spawn_placeholder_screen)
        .add_systems(Update, pulse_placeholder_screen)
        .run();
}

/// The SNES's real background is 256x224 (NTSC); this quad is sized the same purely so a real
/// framebuffer texture can drop in at the same dimensions later without also having to rework
/// the camera/scaling setup.
const SNES_SCREEN_WIDTH: f32 = 256.0;
const SNES_SCREEN_HEIGHT: f32 = 224.0;

fn spawn_placeholder_screen(mut commands: Commands) {
    commands.spawn(Camera2d);

    let entity =
        commands.spawn((Sprite::from_color(Color::srgb(0.2, 0.2, 0.3), Vec2::new(SNES_SCREEN_WIDTH, SNES_SCREEN_HEIGHT)), Transform::default())).id();

    commands.insert_resource(PlaceholderScreen { entity });
}

/// Just proves the window is actually alive and redrawing, nothing more — a gentle color pulse
/// standing in for "a real framebuffer changing every frame." Delete this the moment a real core
/// is driving `Sprite`'s texture instead of its plain color.
fn pulse_placeholder_screen(placeholder: Res<PlaceholderScreen>, time: Res<Time>, mut sprites: Query<&mut Sprite>) {
    let Ok(mut sprite) = sprites.get_mut(placeholder.entity) else { return };
    let pulse = (time.elapsed_secs().sin() * 0.5) + 0.5;
    sprite.color = Color::srgb(0.2, 0.2 + pulse * 0.3, 0.3);
}
