// Re-export bevy::window and bevy::winit for convenience.
pub use bevy::window::*;

//--
use bevy::app::prelude::*;

use bevy::ecs::prelude::*;

use bevy::input::ButtonInput;
use bevy::input::prelude::KeyCode;

use bevy::winit::WakeUp;
use bevy::winit::WinitPlugin;
use bevy::winit::WinitWindows;

use bevy::color::Color;
use bevy::render::camera::ClearColor;
use raw_window_handle::HasWindowHandle;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::WindowId;

//---
/// TODO
#[derive(Clone)]
pub struct WindowPlugin {
    /// TODO
    window_tpl: Option<Window>,
    
    /// TODO
    close_when_requested: bool,
    
    /// TODO
    exit_condition: ExitCondition,
}

impl WindowPlugin {
    /// TODO
    pub fn new<T: Into<String>, R: Into<WindowResolution>>(kind: WindowKind, title: T, size: R) -> Self {
        // TODO: Get window clear color, theme, etc from the supplied StyleGuide.
        
        WindowPlugin {
            window_tpl: Some(Window {
                title: title.into(),
                window_theme: Some(WindowTheme::Dark),
                visible: true,
                present_mode: PresentMode::AutoVsync,
                mode: match kind {
                    WindowKind::Overlay => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    WindowKind::Wallpaper => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    _ => WindowMode::Windowed,
                },
                position: WindowPosition::Centered(MonitorSelection::Current),
                resolution: match kind {
                    WindowKind::Standard => size.into(),
                    _ => WindowResolution::default()
                },
                window_level: match kind {
                    WindowKind::Overlay => WindowLevel::AlwaysOnTop,
                    WindowKind::Wallpaper => WindowLevel::AlwaysOnBottom,
                    _ => WindowLevel::Normal,
                },
                enabled_buttons: EnabledButtons {
                    minimize: true,
                    maximize: true,
                    close: true,
                },
                decorations: match kind {
                    WindowKind::Standard => true,
                    _ => false,
                },
                transparent: match kind {
                    WindowKind::Standard => false,
                    _ => true,
                },
                resizable: match kind {
                    WindowKind::Standard => true,
                    _ => true,
                },
                focused: match kind {
                    WindowKind::Standard => false,
                    _ => true,
                },
                canvas: Some(String::from("slate-surface")),
                prevent_default_event_handling: false,
                fit_canvas_to_parent: true,
                ..Default::default()
            }),
            exit_condition: ExitCondition::DontExit,
            close_when_requested: true,
        }
    }
}

impl Plugin for WindowPlugin {
    /// TODO
    fn build(&self, app: &mut App) {
        
        app.insert_resource(ClearColor(Color::srgb(0.09, 0.09, 0.09)));
        
        app.add_plugins(bevy::window::WindowPlugin {
            primary_window: self.window_tpl.to_owned(),
            exit_condition: self.exit_condition.to_owned(),
            close_when_requested: self.close_when_requested,
        });
        
        app.add_plugins(WinitPlugin::<bevy::winit::WakeUp>::default());
        
        app.add_event::<WindowEvent>();
        
        app.add_systems(bevy::app::Update, bootstrap_new_windows);
        app.add_systems(bevy::app::Update, pair_parent_child_windows);
        app.add_systems(bevy::app::Update, route_window_events);
        app.add_systems(bevy::app::Update, toggle_fullscreen);
        app.add_systems(bevy::app::Update, toggle_decorations);
    }
}

#[derive(Component)]
pub struct ChildWindow {
    parent: Entity,
}

/// TODO
pub fn bootstrap_new_windows(
    mut windows: Query<Entity, (Added<Window>, Without<ChildWindow>)>,
    mut commands: Commands,
) {
    for entity in windows.iter_mut() {
        let mut child_window = Window {
            visible: false,
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
            transparent: true,
            ..Default::default()
        };
        
        child_window.set_maximized(true);
        
        commands.spawn(child_window).insert(ChildWindow { parent: entity });
    }
}

/// TODO
pub fn pair_parent_child_windows(
    mut windows: Query<(Entity, &ChildWindow), Added<ChildWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    for (child_entity, ChildWindow { parent: parent_entity }) in windows.iter_mut() {
        let Some(mut child_window) = winit_windows.get_window(child_entity) else {
            return tracing::warn!("Couldn't get child window for entity#{:}", child_entity)
        };
        
        let Ok(child_handle) = child_window.window_handle().map_err(capture) else {
            return tracing::warn!("Failed to get child window handle!");
        };
        
        let Some(parent_window) = winit_windows.get_window(*parent_entity) else {
            return tracing::warn!("Couldn't get parent window for entity#{:}", child_entity)
        };
        
        let Ok(parent_handle) = parent_window.window_handle().map_err(capture) else {
            return tracing::warn!("Failed to get parent window handle!");
        };
        
        #[cfg(target_os = "windows")] {
            use raw_window_handle::RawWindowHandle;
            use winapi::um::winuser::SetParent;
            use winapi::shared::windef::HWND;
            
            let RawWindowHandle::Win32(win32_child_handle) = child_handle.as_raw() else {
                return tracing::error!("Failed to get raw window handle!");
            };
            
            let RawWindowHandle::Win32(win32_parent_handle) = parent_handle.as_raw() else {
                return tracing::error!("Failed to get raw window handle!");
            };
            
            let child_hwnd: HWND = win32_child_handle.hwnd.get() as HWND;
            let parent_hwnd: HWND = win32_parent_handle.hwnd.get() as HWND;
            
            unsafe {
                SetParent(child_hwnd, parent_hwnd);
                child_window.set_visible(true);
                tracing::debug!("Paired window parent {:?} to child {:?}!", parent_hwnd, child_hwnd)
            }
        }
        
        #[cfg(target_os = "macos")] {
            // TODO
        }
        
        #[cfg(target_os = "linux")] {
            // TODO
        }
    }
}

/// TODO
pub fn bootstrap_child_windows(
    mut windows: Query<(Entity, &ChildWindow), Added<ChildWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    for (child_entity, child_window) in windows.iter_mut() {
        let Some(parent_window) = winit_windows.get_window(child_window.parent) else {
            return tracing::warn!("Couldn't get parent window for entity#{:}", child_entity)
        };
        
        let Ok(handle) = parent_window.window_handle().map_err(capture) else {
            return tracing::warn!("Failed to get parent window handle!");
        };
        
        let child_window_attrs = unsafe {
            create_child_window_attributes(parent_window)
                .with_parent_window(Some(handle.as_raw()))
                // .with_owner_window(parent)
        };
        
        tracing::debug!("Creating new child window with attributes: {:#?}", child_window_attrs);
    }
}

fn create_child_window_attributes(parent_window: &winit::window::Window) -> winit::window::WindowAttributes {
    let parent_position = parent_window.outer_position().unwrap_or(dpi::PhysicalPosition::default());
    let parent_size = parent_window.outer_size();

    let child_x = parent_position.x + 10; // Slight offset
    let child_y = parent_position.y + 10; // Slight offset
    
    unsafe {
        winit::window::Window::default_attributes()
            .with_position(dpi::LogicalPosition::new(child_x, child_y))
            .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
            .with_active(true)
            .with_visible(true)
            .with_inner_size(parent_size)
            .with_maximized(true)
            .with_cursor(winit::window::CursorIcon::Grab)
            // .with_title_background_color(None)
            // .with_title_text_color(None)
            .with_title("Child Window")
    }
}

fn capture<E: std::error::Error>(error: E) {
    tracing::error!("Welp: {:}", error);
}

/// TODO
pub fn route_window_events(
    mut window_evt: EventReader<WindowEvent>,
    mut windows: Query<(Entity, &mut Window)>,
    _: NonSend<WinitWindows>,
) {
    for event in window_evt.read() {
        match event {
            WindowEvent::SetWindowLevel(level) => {
                for (entity, mut window) in windows.iter_mut() {
                    if window.focused {
                        tracing::debug!("Setting window {:} level to {:?}", entity, level);
                        window.window_level = *level;
                    }
                }
            }
            WindowEvent::SetScaleFactor(action) => {
                for (entity, mut window) in windows.iter_mut() {
                    if window.focused {
                        tracing::debug!("Setting window {:} scale factor to {:?}", entity, action);
                        match action {
                            ScaleFactorAction::Reset => {
                                window.resolution.set_scale_factor_override(None);
                            }
                            ScaleFactorAction::Adjust(difference) => {
                                let current_scale_factor = window.resolution.scale_factor_override().unwrap_or(window.scale_factor() as f32);
                                let new_scale_factor = (current_scale_factor + difference).clamp(0.5, 3.0);
                                window.resolution.set_scale_factor_override(Some(new_scale_factor));
                            }
                        }
                    }
                }    
            }
            WindowEvent::Close => {
                for (entity, mut window) in windows.iter_mut() {
                    if window.focused {
                        //..
                    }
                }    
            }
        }
    }
}

/// TODO
pub fn toggle_decorations(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::F10) {
        if let Ok(mut window) = windows.get_single_mut() {
            window.decorations = !window.decorations;
        }
    }
}

/// TODO
pub fn toggle_fullscreen(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::F11) {
        if let Ok(mut window) = windows.get_single_mut() {
            match window.mode {
                WindowMode::Windowed => {
                    window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
                }
                _ => {
                    window.mode = WindowMode::Windowed;
                }
            }
        }
    } else if input.just_pressed(KeyCode::F12) {
        if let Ok(mut window) = windows.get_single_mut() {
            window.decorations = !window.decorations;
        }
    }
}

//---
/// TODO
#[derive(Event, Debug, Clone)]
pub enum WindowEvent {
    /// TODO
    SetWindowLevel(WindowLevel),
    SetScaleFactor(ScaleFactorAction),
    Close,
}

//---
/// TODO
#[derive(Default, Copy, Clone)]
pub enum WindowKind {
    #[default]
    Standard,
    Overlay,
    Wallpaper,
}

#[derive(Debug, Clone)]
pub enum ScaleFactorAction {
    Reset,
    Adjust(f32),
}
