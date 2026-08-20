use std::collections::HashMap;
use std::cell::OnceCell;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

use bevy::math::Rect;
use bevy::math::Vec2;
use bevy::ui::ComputedNode;
use uuid::Uuid as UUID;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::bounded;

use webview2::Controller;
use webview2::EnvironmentBuilder;

use winapi::um::winuser::GetClientRect;
use winapi::shared::windef::RECT;

use raw_window_handle::RawWindowHandle;

use dpi::LogicalPosition;

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
use bevy::prelude::GlobalTransform;
use bevy::prelude::Transform;
use bevy::app::SpawnScene;
use bevy::app::App;
use bevy::app::Plugin;
use bevy::app::Startup;
use bevy::app::PreUpdate;
use bevy::app::Update;
use bevy::app::Last;
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
use bevy::asset::AssetServer;
use bevy::color::Alpha;
use bevy::color::Color;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::ui::AlignItems;
use bevy::ui::BackgroundColor;
use bevy::ui::Interaction;
use bevy::ui::JustifyContent;
use bevy::ui::Val;
use bevy::ui::Node;
use bevy::ui::UiRect;
use bevy::window::Window;
use bevy::window::PrimaryWindow;
use bevy::window::WindowResized;
use bevy::winit::WinitWindows;

//--
#[derive(oops::Error)]
pub enum WebViewError {
    #[msg("missing expected webview for entity {:0}")]
    Missing(Entity),
}

#[derive(Default)]
pub struct WebViewPlugin {
    user_dir: PathBuf,
}

impl Plugin for WebViewPlugin {
    // TODO
    fn build(&self, application: &mut App) {
        // TODO: DO NOT leak the user_dir here ..
        let home_dir = dirs::home_dir().expect("failed to get home dir");
        let user_dir = home_dir.join("./.slate");
        
        let (event_tx, event_rx) = bounded(100);
        let (command_tx, command_rx) = bounded(100);
        
        application
            .insert_non_send_resource(WebViewProvider::new(user_dir, command_tx, event_rx))
            .add_event::<WebViewEvent>()
            .add_systems(Last, spawn_webview_controllers)
            .add_systems(Last, handle_webview_events)
            .add_systems(Last, sync_window_resize);
    }
}

/// TODO
pub struct WebViewProvider<'env> {
    /// TODO
    env_builder: EnvironmentBuilder<'env>,
    
    /// TODO
    controllers: HashMap<Entity, Arc<OnceCell<Controller>>>,
    
    /// TODO
    command_tx: Sender<WebViewCommand>,
    
    /// TODO
    events_rx: Receiver<WebViewEvent>,
}

impl WebViewProvider<'_> {
    /// TODO
    fn new<P: AsRef<Path>>(user_dir: P, command_tx: Sender<WebViewCommand>, events_rx: Receiver<WebViewEvent>) -> Self {
        // TODO: Clean up the leak (either in a Drop impl or by keeping
        //  the `PathBuf` in long-term memory somewhere else).
        // For now, it doesn't matter because (under normal conditions)
        //  the program will exit before the `WebViewProvider` is dropped.
        let user_dir = Box::leak(Box::new(user_dir.as_ref().to_path_buf()));
        
        let env_builder = EnvironmentBuilder::new()
            .with_user_data_folder(user_dir)
            .with_additional_browser_arguments("")
            .with_allow_single_sign_on_using_osprimary_account(true);
        
        WebViewProvider {
            env_builder,
            controllers: HashMap::new(),
            command_tx,
            events_rx,
        }
    }
}

//---
/// TODO
#[derive(Component, Default, Debug)]
pub struct WebViewDisplay {
    /// TODO
    address: Option<String>,
}

impl WebViewDisplay {
    /// TODO
    pub fn new() -> Self {
        WebViewDisplay {
            address: None,
        }
    }
}

impl WebViewDisplay {
    /// TODO
    pub fn set_bounds(&mut self, rect: RECT) -> Result<(), ()> {
        Ok(()) // TODO
    }
    
    /// TODO
    pub fn with_address<S: Into<String>>(mut self, address: S) -> Result<Self, ()> {
        self.address = Some(address.into());
        Ok(self) // TODO
    }
    
    /// TODO
    pub fn set_address<S: Into<String>>(&mut self, address: S) -> Result<(), ()> {
        self.address = Some(address.into());
        Ok(()) // TODO
    }
    
    /// TODO
    pub fn notify_parent_window_position_changed(&mut self) -> () {
        // tracing::debug!("Notify webview window position change ..");
    }
}

// System to spawn controllers for new WebViewElements
pub fn spawn_webview_controllers(
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
    mut webview_provider: NonSendMut<WebViewProvider>,
    mut webviews: Query<(Entity, &Node, &ComputedNode, &GlobalTransform, &Transform, &WebViewDisplay), Added<WebViewDisplay>>,
    mut commands: Commands,
) {
    use raw_window_handle::RawWindowHandle;
    use raw_window_handle::HasWindowHandle;
    
    for (webview_entity, node, computed_node, global, local, display) in webviews.iter_mut() {
        #[cfg(feature="verbose")]
        tracing::debug!("Spawning WebViewDisplay controller!");
        
        if let Ok((window_entity, window)) = windows.get_single() {
            let controller = Arc::new(OnceCell::new());
            
            use winapi::shared::windef::HWND;
            
            // TODO: Get the HWND for the window.
            let winit_window = winit_windows.get_window(window_entity).expect("winit window");
            let window_handle = winit_window.window_handle().expect("window handle").as_raw();
            
            let RawWindowHandle::Win32(win32_window_handle) = window_handle else {
                return tracing::error!("Failed to get raw window handle!");
            };
            
            let raw_window_handle = win32_window_handle.hwnd.get() as HWND;
            
            #[cfg(all(feature="debug", feature="verbose"))]
            tracing::debug!("Raw window handle: {:?}", raw_window_handle);
            
            if raw_window_handle.is_null() {
                return tracing::error!("Invalid HWND for WebView2 controller");
            }
            
            let controller_clone = controller.clone();
            
            let address = display.address.clone().unwrap_or_default();
            let rect = Rect::from_center_size(global.translation().truncate(), computed_node.size());
            
            #[cfg(all(feature="debug", feature="verbose"))]
            tracing::debug!("Setting webview position to: {:#?}", rect);
            
            if let Err(error) = webview_provider.env_builder.build(move |env| {
                env?.create_controller(raw_window_handle, move |controller| {
                    let controller = controller?;
                    let controller2 = controller.get_controller2()?;
                    let webview = controller.get_webview()?;
                    
                    controller.add_accelerator_key_pressed(|_, _| {
                        tracing::trace!("Accelerator key pressed!");
                        Ok(())
                    });
                    
                    webview.add_contains_full_screen_element_changed(|_| {
                        tracing::trace!("Wants fullscreen!");
                        Ok(())
                    });
                    
                    webview.add_window_close_requested(|_| {
                        tracing::trace!("Window Close Requested!");
                        Ok(())
                    });
                    
                    webview.execute_script("console.log('hello from webview2 setup!');", |result_json| {
                        tracing::trace!("Script Executed: {:}", result_json);
                        Ok(())
                    });
                    
                    webview.add_document_title_changed(|webview| {
                        tracing::trace!("Document title changed: {:?}", webview.get_document_title());
                        Ok(())
                    });
                    
                    webview.add_content_loading(|_, args| {
                        tracing::trace!("Content loading: {:?}", args);
                        Ok(())
                    });
                    
                    webview.add_navigation_starting(|_, args| {
                        tracing::trace!("Navigation starting: {:?}", args.get_uri()?);
                        Ok(())
                    });
                    
                    webview.add_source_changed(|_, args| {
                        tracing::trace!("Source changed: {:?}", args);
                        Ok(())
                    });
                    
                    webview.add_history_changed(|_| {
                        tracing::trace!("History changed!");
                        Ok(())
                    });
                    
                    webview.add_navigation_completed(|_, args| {
                        tracing::trace!("Navigation completed: {:?}", args.get_is_success()?);
                        Ok(())
                    });
                    
                    // controller2.put_default_background_color(webview2_sys::Color {
                    //     a: 0,
                    //     r: 0,
                    //     g: 0,
                    //     b: 0,
                    // });
                    
                    let settings = webview.get_settings()?;
                    
                    settings.put_is_script_enabled(true)?;
                    settings.put_are_default_context_menus_enabled(true)?;
                    settings.put_is_status_bar_enabled(false)?;
                    settings.put_is_zoom_control_enabled(false)?;
                    settings.put_is_zoom_control_enabled(true)?;
                    
                    let bounds = unsafe {
                        let mut rect = core::mem::zeroed();
                        GetClientRect(raw_window_handle, &mut rect);
                        rect
                    };
                    
                    // controller.put_bounds(bounds);
                    controller.put_bounds(RECT {
                        top: rect.min.y as i32,
                        left: rect.min.x as i32,
                        right: rect.max.x as i32,
                        bottom: rect.max.y as i32,
                    })?;
                    
                    webview.navigate(&address)?;
                    
                    tracing::trace!("Added webview: {:?}", controller);
                    controller_clone.set(controller).expect("set controller");
                    
                    Ok(()) // <3
                })
            }) {
                // Handle errors (e.g., log an error message)
                tracing::error!("Couldn't create WebView2 controller for entity {:?}: {:?}", webview_entity, error);
            }
            
            // Store the controller in the provider's HashMap
            webview_provider.controllers.insert(webview_entity, controller);
        }
    }
}

pub fn sync_window_resize(
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
    mut webviews: Query<(Entity, &GlobalTransform, &Node, &ComputedNode, &mut WebViewDisplay)>,
    mut webview_provider: NonSendMut<WebViewProvider>,
    mut resize_evt: EventReader<WindowResized>,
    mut commands: Commands,
) {
    if resize_evt.len() > 0 {
        resize_evt.clear();
        
        use raw_window_handle::RawWindowHandle;
        use raw_window_handle::HasWindowHandle;
        
        for (webview_entity, transform, node, computed_node, mut webview) in webviews.iter_mut() {
            if let Ok((window_entity, window)) = windows.get_single() {
                use winapi::shared::windef::HWND;
                
                // TODO: Get the HWND for the window.
                let winit_window = winit_windows.get_window(window_entity).expect("winit window");
                let window_handle = winit_window.window_handle().expect("window handle").as_raw();
                
                let RawWindowHandle::Win32(win32_window_handle) = window_handle else {
                    return tracing::error!("Failed to get raw window handle!");
                };
                
                let raw_window_handle = win32_window_handle.hwnd.get() as HWND;
                
                #[cfg(feature="verbose")]
                tracing::debug!("Raw window handle: {:?}", raw_window_handle);
                
                if raw_window_handle.is_null() {
                    return tracing::error!("Invalid HWND for WebView2 controller");
                }
                
                if let Some(webview_controller) = webview_provider.controllers.get(&webview_entity) {
                    let rect = Rect::from_center_size(transform.translation().truncate(), computed_node.size());
                    
                    if let Some(controller_lock) = webview_controller.get() {
                        if let Err(error) = controller_lock.put_bounds(RECT {
                            top: rect.min.y as i32,
                            left: rect.min.x as i32,
                            right: rect.max.x as i32,
                            bottom: rect.max.y as i32,
                        }) {
                            // TODO: Better error recovery here.
                            tracing::error!("Failed to resize webview: {:?}", error)
                        }
                    }
                }
            }
        }
    }
}

fn position_to_rect(window: (), pos: bevy::prelude::Vec3, size: bevy::prelude::Vec2) -> RECT {
    let bounds = (0, 300, 300, 0);
    
    // Extract window dimensions from the RECT
    let window_width = (bounds.1 - bounds.3) as f32;
    let window_height = (bounds.2 - bounds.0) as f32;
    
    // Calculate RECT coordinates in screen space
    let left = pos.x - (size.x / 2.0);
    let top = pos.y + size.y / 2.0;
    let right = pos.x + (size.x / 2.0);
    let bottom = pos.y - size.y / 2.0;
    
    // Convert to WebView2-compatible RECT with top-left origin
    RECT {
        left: (left + window_width / 2.0) as i32 + bounds.3,
        top: (window_height / 2.0 - top) as i32 + bounds.0,
        right: (right + window_width / 2.0) as i32 + bounds.3,
        bottom: (window_height / 2.0 - bottom) as i32 + bounds.0,
    }
}

/// TODO
fn handle_webview_events(
    webview_provider: NonSend<WebViewProvider>,
    mut webview_evt: EventWriter<WebViewEvent>,
) {
    // tracing::trace!("Watching for webview events ..");
    
    while let Ok(message) = webview_provider.events_rx.recv() {
        tracing::debug!("Got message from WebView Channel: {:#?}", message);
        match message {
            // TODO
            _ => {
                webview_evt.send(message);
            },
        }
    }
}

//---
/// TODO
enum WebViewCommand {
    /// TODO
    CreateWebView,
    
    /// TODO
    UpdateBounds,
}

//---
// Define a custom Bevy event type (optional)
#[derive(Event, Debug)]
enum WebViewEvent {
    /// TODO
    PageNavigated(String),
}
