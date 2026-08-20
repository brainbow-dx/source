//! A real, working UI scene built against the current `escher-core` — the successor to the
//! pre-refactor `slate`-era `examples/basic.rs` (kept for reference, non-functional, at
//! `src/legacy/basic.rs.example`). That example depended on a `chizel::uix!`/`chizel::styles!`
//! macro DSL and a `Div`/`Header`/`Footer`/`Sidebar`/`TextBlock`/`Button` component library that
//! no longer exist. This one uses only what current `escher-core` actually provides: the
//! `Scaffold` builder API directly (`style`/`slot`/`content`, the same idiom
//! `escher-terminal`'s `examples/ratatui.rs` already uses), `Container`/`Text` elements, and
//! `Header`/`Body`/`Footer`/`Legend`/`Content` as plain slot-marker types — no macros, no bespoke
//! component library, and it renders through the new `escher_bevy::surface::BevySurface`.
//!
//! Redraws every 2 seconds via `DrawTimer` (like `basic.rs` did) so the "rebuild the whole
//! subtree on every draw" strategy documented in `surface.rs` is actually exercised, not just
//! run once at startup.

use core::time::Duration;

use bevy::prelude::*;

use escher_bevy::surface::BevySurface;
use escher_bevy::time::sync_draw_timer;
use escher_bevy::time::DrawTimer;
use escher_bevy::time::DrawTimerFinishedEvent;
use escher_bevy::EscherBevyConfig;
use escher_bevy::EscherBevyPlugin;

use escher_core::element::Body;
use escher_core::element::Content as ContentSlot;
use escher_core::element::Footer;
use escher_core::element::Header;
use escher_core::element::Legend;
use escher_core::scaffold::Scaffold;
use escher_core::style::BackgroundColor as ScaffoldBackgroundColor;
use escher_core::style::Border;
use escher_core::style::BorderStyle;
use escher_core::style::ContentColor;
use escher_core::style::FlexDirection as ScaffoldFlexDirection;
use escher_core::style::Gap;
use escher_core::style::Padding;
use escher_core::style::Size;
use escher_core::style::Unit;
use escher_core::style::Value;

fn main() {
    App::new()
        .add_plugins(EscherBevyPlugin::new(
            EscherBevyConfig::default().with_clear_color(Color::hsla(220.0, 0.11, 0.11, 1.0)),
        ))
        .init_resource::<SceneSurface>()
        .add_message::<DrawTimerFinishedEvent>()
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, sync_draw_timer)
        .add_systems(Update, draw_scene)
        .run();
}

#[derive(Resource, Default)]
struct SceneSurface(BevySurface);

#[derive(Component)]
struct SceneRoot;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        SceneRoot,
    ));

    commands.spawn(DrawTimer::new(Duration::from_secs(2)));
}

fn draw_scene(
    mut surface: ResMut<SceneSurface>,
    root_query: Query<Entity, With<SceneRoot>>,
    mut timer_finished_evtr: MessageReader<DrawTimerFinishedEvent>,
    mut commands: Commands,
) {
    for _ in timer_finished_evtr.read() {
        let Ok(root) = root_query.single() else {
            continue;
        };

        tracing::debug!("Redrawing scene ..");
        surface.0.draw(&mut commands, root, build_scene);
    }
}

fn build_scene(root: Scaffold) -> Scaffold {
    root.style(ScaffoldFlexDirection::Column)
        .slot::<Header>(|header| {
            header
                .style(ScaffoldFlexDirection::Row)
                .style(Size::height(60))
                .style(Padding::new(10))
                .style(ScaffoldBackgroundColor::from("#2a2a2aff"))
                .style(ContentColor::from("#eeeeeeff"))
                .content(Some("Escher / Bevy — Simple Scene"))
        })
        .slot::<Body>(|body| {
            body.style(ScaffoldFlexDirection::Row)
                .style(Gap(10.into()))
                .style(Padding::new(10))
                .slot::<Legend>(|sidebar| {
                    sidebar
                        .style(Size::width(200))
                        .style(Padding::new(10))
                        .style(ScaffoldBackgroundColor::from("#668866ff"))
                        .style(ContentColor::from("#111111ff"))
                        .content(Some("Sidebar"))
                })
                .slot::<ContentSlot>(|content| {
                    content
                        .style(Size(Value::Percent(Unit::from(100)), Value::Auto, Value::Auto))
                        .style(Padding::new(10))
                        .style(Border::new(2, BorderStyle::Solid, Some("#88aa88ff".into())))
                        .style(ScaffoldBackgroundColor::from("#333344ff"))
                        .style(ContentColor::from("#eeeeeeff"))
                        .content(Some("Main content area — redraws every 2 seconds."))
                })
        })
        .slot::<Footer>(|footer| {
            footer
                .style(ScaffoldFlexDirection::Row)
                .style(Size::height(40))
                .style(Padding::new(10))
                .style(ScaffoldBackgroundColor::from("#886666ff"))
                .style(ContentColor::from("#eeeeeeff"))
                .content(Some("Footer"))
        })
}
