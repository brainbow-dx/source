use std::collections::HashMap;
use std::cell::OnceCell;
use std::sync::Arc;

use bevy::app::SpawnScene;
use bevy::asset::AssetServer;
use bevy::color::Alpha;
use bevy::color::Color;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::BuildChildren;
use bevy::prelude::Button;
use bevy::prelude::ButtonBundle;
use bevy::prelude::Changed;
use bevy::prelude::Children;
use bevy::prelude::IntoSystem;
use bevy::prelude::KeyCode;
use bevy::prelude::NodeBundle;
use bevy::prelude::Res;
use bevy::prelude::ResMut;
use bevy::prelude::Resource;
use bevy::prelude::TextBundle;
use bevy::ui::AlignItems;
use bevy::ui::BackgroundColor;
use bevy::ui::Interaction;
use bevy::ui::JustifyContent;
use bevy::ui::Val;
use bevy::window::Window;
use dpi::LogicalPosition;
use uuid::Uuid as UUID;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::bounded;

use webview2::Controller;
use webview2::EnvironmentBuilder;

use winapi::um::winuser::GetClientRect;
use winapi::shared::windef::RECT;

use raw_window_handle::RawWindowHandle;

use bevy::app::App;
use bevy::app::Plugin;
use bevy::app::Startup;
use bevy::app::PreUpdate;
use bevy::app::Update;
use bevy::app::Last;
use bevy::prelude::GlobalTransform;
use bevy::prelude::Transform;
use bevy::ui::Node;
// use bevy::ui::Node;
use bevy::ui::UiRect;
use bevy::ecs::prelude::Entity;
use bevy::ecs::query::Added;
use bevy::ecs::system::NonSend;
use bevy::ecs::prelude::Event;
use bevy::ecs::prelude::EventReader;
use bevy::ecs::prelude::EventWriter;
use bevy::ecs::component::Component;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Query;
use bevy::ecs::system::NonSendMut;
use bevy::ecs::query::With;
use bevy::window::PrimaryWindow;
use bevy::window::WindowResized;
use bevy::winit::WinitWindows;

//--
#[derive(oops::Error)]
pub enum InputError {
    #[msg("uhh")]
    Unknown
}

#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    // TODO
    fn build(&self, app: &mut App) {
        app.add_event::<KeyboardInput>();
        
        app.insert_resource(FocusedInput::default());
        
        app.add_systems(Update, text_input_focus_system);
        app.add_systems(Update, text_input_capture_system);
        app.add_systems(Update, text_input_defocus_system);
    }
}

// Component to identify text input fields
#[derive(Component)]
pub struct TextInput;

#[derive(Resource, Default)]
pub struct FocusedInput(Option<Entity>);

pub fn text_input_focus_system(
    mut focused_input: ResMut<FocusedInput>,
    mut interaction_query: Query<
        (Entity, &Interaction),
        (Changed<Interaction>, With<TextInput>),
    >,
    mut query: Query<
        (&Node, &mut BackgroundColor, &TextInput, &Children),
        With<Button>,
    >,
) {
    for (entity, interaction) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if let Ok((node, mut bg_color, text_input, children)) = query.get_mut(entity) {
                    if let Some(text_entity) = children.get(0) {
                        tracing::trace!("Setting focus to text entity#{:}", text_entity);
                        focused_input.0 = Some(*text_entity);
                        // *bg_color = Color::srgb(0.5, 0.5, 0.5).into();
                    }
                }
            }
            Interaction::Hovered => {
                // if let Ok((node, mut bg_color, text_input, children)) = query.get_mut(entity) {
                //     if let Some(text_entity) = children.get(0) {
                //         if Some(*text_entity) != focused_input.0 {
                //             *bg_color = Color::srgb(0.5, 0.5, 0.5).into();
                //         }
                //     }
                // }
            }
            Interaction::None => {
                // if let Ok((node, mut bg_color, text_input, children)) = query.get_mut(entity) {
                //     *bg_color = Color::NONE.into();
                // }
            }
        }
    }
}

pub fn text_input_defocus_system(
    mouse_button_input: Res<bevy::input::ButtonInput<bevy::prelude::MouseButton>>,
    mut focused_input: ResMut<FocusedInput>,
    interaction_query: Query<&Interaction, With<TextInput>>,
) {
    if mouse_button_input.just_pressed(bevy::prelude::MouseButton::Left) {
        if !interaction_query.iter().any(|i| *i == Interaction::Pressed) {
            tracing::debug!("De-focused text input!");
            focused_input.0 = None;
        }
    }
}

pub fn text_input_capture_system(
    focused_input: Res<FocusedInput>,
    mut char_evr: EventReader<KeyboardInput>,
    mut query: Query<&mut bevy::ui::widget::Text>,
) {
    if let Some(focused_input) = focused_input.0 {
        // Get the single Text component we have
        if let Ok(mut text) = query.get_mut(focused_input) {
            // Iterate over all received characters
            for ev in char_evr.read() {
                #[cfg(all(feature="verbose", feature="inspect"))]
                tracing::debug!("Sending input to entity#{:}: {:#?}", focused_input, ev);
                
                let len = text.0.len();
                let (min_len, max_len) = (0, 120);
                
                match ev.logical_key {
                    Key::Character(ref character) if ev.state == ButtonState::Pressed && (len + character.len()) < max_len => {
                        for character in character.chars() {
                            text.0.push(character);
                        }
                    }
                    Key::Space if ev.state == ButtonState::Pressed && (len + 1) < max_len => {
                        text.0.push(' ');
                    }
                    Key::Backspace if ev.state == ButtonState::Pressed && len > min_len => {
                        text.0.pop();
                    }
                    _ => {
                        // TODO
                    }
                }
            }
        }
    }
}
