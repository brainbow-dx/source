use bevy::app::prelude::*;
use bevy::ecs::prelude::*;

use bevy::math::Vec2;
use bevy::math::Vec3;

use bevy::camera::Camera2d;

use bevy::color::Color;

use bevy::transform::prelude::*;

use bevy::window::PrimaryWindow;
use bevy::window::Window;
use winit::window::CursorIcon;

use bevy::gizmos::gizmos::Gizmos;

pub struct ReticlePlugin;

impl ReticlePlugin {
    pub fn new() -> Self {
        ReticlePlugin
    }
}

impl Default for ReticlePlugin {
    fn default() -> Self {
        ReticlePlugin
    }
}

impl Plugin for ReticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_reticles);
        app.add_systems(PreUpdate, sync_reticle_display);
        app.add_systems(Update, sync_reticle_position);
        app.add_systems(Update, sync_reticle_cursor);
    }
}

fn setup_reticles(mut commands: Commands) {
    commands.spawn(ReticleBundle::new(
        ReticleShape::Circle,
        ReticleStyle::new(Color::WHITE, 30., Some(CursorIcon::default())),
        ReticlePosition::new(Vec2::ZERO, Vec3::ZERO),
    ));
}

fn sync_reticle_display(
    windows: Query<&Window, With<PrimaryWindow>>,
    reticles: Query<(&ReticleShape, &ReticleStyle, &Transform), With<ReticleShape>>,
    mut gizmos: Gizmos,
) {
    if let Ok(window) = windows.single() {
        if window.cursor_position().is_some() {
            for (shape, style, transform) in reticles.iter() {
                let position = transform.translation.truncate();

                match shape {
                    ReticleShape::Circle => {
                        gizmos.circle_2d(position, style.size, style.color);
                    }
                    ReticleShape::Square => {
                        gizmos.rect_2d(position, Vec2::ONE * style.size, style.color);
                    }
                    ReticleShape::Triangle => {
                        gizmos.circle_2d(position, style.size, style.color).resolution(3);
                    }
                }
            }
        }
    }
}

fn sync_reticle_position(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<&Transform, (With<Camera2d>, Without<ReticlePosition>)>,
    mut reticles: Query<(&mut ReticlePosition, &mut Transform), With<ReticlePosition>>,
) {
    if let Ok(window) = windows.single() {
        if let Some(cursor_pos) = window.cursor_position() {
            let size = Vec2::new(window.width(), window.height());
            let screen_pos = cursor_pos;
            let cursor_pos = cursor_pos - size / 2.0;

            if let Some(camera_transform) = cameras.iter().next() {
                let inverse_camera_matrix = camera_transform.to_matrix().inverse();
                let space_2d_pos = inverse_camera_matrix.transform_point3(Vec3::new(cursor_pos.x, -cursor_pos.y, 0.0));

                for (mut position, mut transform) in reticles.iter_mut() {
                    position.screen = screen_pos;
                    position.space_2d = space_2d_pos;
                    transform.translation = space_2d_pos;
                }
            }
        }
    }
}

/// Sync the currently selected cursor to the appropriate windows.
///
/// TODO (ported from the pre-refactor `slate` prototype): the actual cursor-icon assignment
/// was never wired up here — `window.cursor.icon` doesn't exist on Bevy's `Window` at all in
/// the version this was targeting either, so this system only re-runs on `ReticleStyle` change
/// and otherwise no-ops. Left as a hook for whoever wires up real cursor swapping.
fn sync_reticle_cursor(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    reticles: Query<&ReticleStyle, (With<ReticleShape>, Changed<ReticleStyle>)>,
) {
    for _style in reticles.iter() {
        let _ = windows.single_mut();
    }
}

#[derive(Bundle, Default)]
pub struct ReticleBundle {
    pub kind: ReticleKind,
    pub shape: ReticleShape,
    pub style: ReticleStyle,
    pub position: ReticlePosition,
    pub transform: Transform,
}

impl ReticleBundle {
    fn new(shape: ReticleShape, style: ReticleStyle, position: ReticlePosition) -> Self {
        ReticleBundle {
            kind: ReticleKind::default(),
            shape,
            style,
            position,
            transform: Transform::from_translation(Vec3::ZERO),
        }
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct ReticleStyle {
    color: Color,
    size: f32,
    // Not read yet — see the TODO on `sync_reticle_cursor` above.
    #[allow(dead_code)]
    cursor: Option<CursorIcon>,
}

impl ReticleStyle {
    pub fn new(color: Color, size: f32, cursor: Option<CursorIcon>) -> Self {
        ReticleStyle { color, size, cursor }
    }
}

#[derive(Component, Default, Debug, Clone)]
pub enum ReticleKind {
    #[default]
    Gizmo,
}

#[derive(Component, Default, Debug, Clone)]
pub enum ReticleShape {
    #[default]
    Circle,
    Square,
    Triangle,
}

/// Tracks the position of the user's cursor in screen, 2D, and 3D space.
#[derive(Component, Default, Debug, Clone)]
pub struct ReticlePosition {
    /// The screen-space position of the reticle, corrected for relevant window placement and
    /// camera position.
    pub screen: Vec2,
    /// The 2D space position of the reticle.
    pub space_2d: Vec3,
}

impl ReticlePosition {
    pub fn new(screen: Vec2, space_2d: Vec3) -> Self {
        ReticlePosition { screen, space_2d }
    }
}
