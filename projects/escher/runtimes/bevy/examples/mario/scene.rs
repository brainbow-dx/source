//! An on-demand Bevy-rendered view of the same running game state `render.rs` draws to the
//! terminal — real `Sprite` entities positioned straight from `physics::MarioState`/
//! `ghosts::GhostEntry`, proving the same physics/relay/persistence modules also drive a genuine
//! 2D scene, not just ASCII art. Toggled by `B` (see `main.rs`'s `KeyboardEvent` handler) rather
//! than always-on: opening a real OS window mid-game shouldn't be the default, and rebuilding
//! this every tick only costs anything while a player actually asked to see it.

use bevy::camera::Camera;
use bevy::camera::Camera2d;
use bevy::camera::ClearColorConfig;
use bevy::camera::RenderTarget;
use bevy::color::Color as BevyColor;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::math::Vec2;
use bevy::math::Vec3;
use bevy::sprite::Sprite;
use bevy::transform::components::Transform;
use bevy::window::WindowLevel;
use bevy::window::WindowRef;

use crate::ghosts;
use crate::physics;
use crate::physics::MarioState;
use crate::GameState;

const SCENE_WIDTH: f32 = 960.0;
const SCENE_HEIGHT: f32 = 600.0;
const PLAYER_SIZE: f32 = 28.0;
const GHOST_SIZE: f32 = 10.0;
/// Drawn thickness of the ground line in scene pixels — independent of `MARIO_GROUND_Y`'s own
/// fractional coordinate, which has no thickness at all (it's a single row, not a span). A visible
/// but thin bar, distinct from a platform's own fill so the two never look like the same surface.
const GROUND_THICKNESS: f32 = 8.0;
const GROUND_COLOR: (u8, u8, u8) = (74, 74, 84);
/// Matches `render::PLATFORM_COLOR` so the same surface reads as the same thing in both the
/// terminal and this scene window.
const PLATFORM_COLOR: (u8, u8, u8) = (94, 84, 74);

/// Marks the on-demand scene window and its camera, so `spawn_scene_window_on_toggle` can find
/// and despawn exactly those two entities without touching anything else in the app.
#[derive(Component)]
pub(crate) struct SceneWindowMarker;

/// Marks every sprite this module owns. Despawned and rebuilt from scratch each tick rather than
/// diffed — the same "full rebuild, no diffing" tradeoff `escher_bevy::surface::BevySurface`
/// already makes for its own `Scaffold` rendering, good enough for a debug view redrawing well
/// under a hundred sprites a frame.
#[derive(Component)]
pub(crate) struct SceneSprite;

/// Opens or closes the scene window to match `GameState::bevy_scene_open`, toggled by `main.rs`'s
/// `B` key handler. A no-op most ticks — only acts on the edge where the flag and the window's
/// actual presence disagree.
pub fn spawn_scene_window_on_toggle(mut commands: Commands, state: Res<GameState>, existing: Query<Entity, With<SceneWindowMarker>>) {
    let wants_open = *state.bevy_scene_open.read();
    let is_open = !existing.is_empty();
    if wants_open == is_open {
        return;
    }

    if wants_open {
        let window_entity = commands
            .spawn((SceneWindowMarker, escher_bevy::window::create_window("Mario — Scene", SCENE_WIDTH, SCENE_HEIGHT, true, false, WindowLevel::Normal)))
            .id();
        commands.spawn((
            SceneWindowMarker,
            Camera2d,
            // Matches Anvil's own terminal `background` token (`spec/design/styleguide/anvil.md`)
            // rather than an unrelated ad hoc navy tint, so this window reads as the same app as
            // the terminal it pops up alongside instead of a colder, bluer surface next to it.
            Camera { clear_color: ClearColorConfig::Custom(BevyColor::srgb_u8(0x20, 0x20, 0x20)), ..Default::default() },
            RenderTarget::Window(WindowRef::Entity(window_entity)),
        ));
    } else {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Rebuilds every player/ghost/platform sprite from the same state `terminal_draw`/`draw_frame`
/// already read this tick — skipped entirely while the scene window isn't open, so this costs
/// nothing during ordinary (terminal-only) play.
///
/// Real bug, found live per the user's own report: this used to draw only players and ghosts —
/// never the ground, never a platform. With nothing solid ever rendered, landing on the platform
/// looked like floating at an arbitrary point in empty space ("the center"), and standing on it
/// looked identical to standing on the ground, since neither ever had a visible surface at all.
/// The terminal side already drew both (`render::mario_body_text`'s own platform/backdrop
/// handling); this window just never got the same treatment.
pub fn sync_scene_sprites(
    mut commands: Commands,
    state: Res<GameState>,
    colliders: Query<&physics::MarioCollider>,
    existing: Query<Entity, With<SceneSprite>>,
    scene_window: Query<Entity, With<SceneWindowMarker>>,
) {
    if scene_window.is_empty() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    spawn_ground_sprite(&mut commands);
    for collider in colliders.iter() {
        spawn_platform_sprite(&mut commands, collider.rect);
    }

    for (index, (_, _, mario)) in state.mario.read().iter().enumerate() {
        spawn_player_sprite(&mut commands, mario, physics::mario_player_color(index));
    }

    let elapsed_seconds = state.start.elapsed().saturating_sub(*state.paused_accumulated.read()).as_secs_f32();
    for ghost in state.ghosts.read().iter().rev().take(ghosts::MARIO_GHOST_RENDER_LIMIT) {
        let (x, y) = ghosts::mario_ghost_position(elapsed_seconds, ghost.drift);
        spawn_ghost_sprite(&mut commands, x, y, ghost.color);
    }
}

/// Maps `physics::MarioState`'s fractional, top-left-origin play-area coordinates (`y = 0.0` top,
/// `MARIO_GROUND_Y = 1.0` the ground — see that constant's own doc comment) onto this scene's
/// centered, Y-up Bevy world space. `size` is the sprite's own height: `y` here is meant as a feet/
/// bottom-edge position, and Bevy anchors a sprite at its center, so shifting up by half the
/// sprite's own size is what actually puts its bottom edge at `y` rather than its middle.
fn to_scene_position(x: f32, y: f32, size: f32) -> Vec3 {
    Vec3::new((x - 0.5) * SCENE_WIDTH, (0.5 - y) * SCENE_HEIGHT + size * 0.5, 0.0)
}

fn spawn_player_sprite(commands: &mut Commands, mario: &MarioState, color: (u8, u8, u8)) {
    commands.spawn((
        SceneSprite,
        Sprite { color: BevyColor::srgb_u8(color.0, color.1, color.2), custom_size: Some(Vec2::splat(PLAYER_SIZE)), ..Default::default() },
        Transform::from_translation(to_scene_position(mario.x, mario.y, PLAYER_SIZE)),
    ));
}

fn spawn_ghost_sprite(commands: &mut Commands, x: f32, y: f32, color: (u8, u8, u8)) {
    commands.spawn((
        SceneSprite,
        Sprite { color: BevyColor::srgba_u8(color.0, color.1, color.2, 160), custom_size: Some(Vec2::splat(GHOST_SIZE)), ..Default::default() },
        Transform::from_translation(to_scene_position(x, y, GHOST_SIZE)),
    ));
}

/// A full-width bar at `MARIO_GROUND_Y`, the walking surface every player and platform's own
/// height is measured relative to — until now, never actually drawn in this window.
fn spawn_ground_sprite(commands: &mut Commands) {
    commands.spawn((
        SceneSprite,
        Sprite { color: BevyColor::srgb_u8(GROUND_COLOR.0, GROUND_COLOR.1, GROUND_COLOR.2), custom_size: Some(Vec2::new(SCENE_WIDTH, GROUND_THICKNESS)), ..Default::default() },
        // Centered on the ground line itself (`size: 0.0`, no bottom-edge shift) rather than
        // treated as a bottom-anchored sprite the way a player is.
        Transform::from_translation(to_scene_position(0.5, physics::MARIO_GROUND_Y, 0.0)),
    ));
}

/// One static platform, drawn as a solid bar spanning its own collider rect exactly — `rect`'s
/// fractional `(x0, y0, x1, y1)` maps directly onto scene pixels, centered on the rect's own
/// midpoint, not bottom-anchored the way a player sprite is.
fn spawn_platform_sprite(commands: &mut Commands, rect: (f32, f32, f32, f32)) {
    let (x0, y0, x1, y1) = rect;
    let width = (x1 - x0) * SCENE_WIDTH;
    let height = (y1 - y0) * SCENE_HEIGHT;
    let center_x = (x0 + x1) * 0.5;
    let center_y = (y0 + y1) * 0.5;
    commands.spawn((
        SceneSprite,
        Sprite { color: BevyColor::srgb_u8(PLATFORM_COLOR.0, PLATFORM_COLOR.1, PLATFORM_COLOR.2), custom_size: Some(Vec2::new(width, height)), ..Default::default() },
        Transform::from_translation(to_scene_position(center_x, center_y, 0.0)),
    ));
}
