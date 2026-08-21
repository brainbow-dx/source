//! Thin Bevy glue for `escher-os` — sets the standard application menu bar on startup. Dialogs
//! and clipboard access need no Bevy-specific wiring at all (they're synchronous, stateless AppKit
//! calls with nothing to tie to the ECS) — call `escher_os::dialog::*`/`escher_os::clipboard::*`
//! directly wherever needed.

use bevy::app::App;
use bevy::app::Plugin;
use bevy::app::Startup;

pub struct OsPlugin {
    app_name: String,
    extra_menu_items: Vec<escher_os::menu::MenuItem>,
}

impl OsPlugin {
    pub fn new(app_name: impl Into<String>) -> Self {
        OsPlugin { app_name: app_name.into(), extra_menu_items: Vec::new() }
    }

    /// Appended to the menu bar after the standard App/Edit menus `default_application_menu`
    /// already provides — a consumer's own custom-action submenu(s), e.g. Anvil's demo menu.
    pub fn with_extra_menu_items(mut self, items: Vec<escher_os::menu::MenuItem>) -> Self {
        self.extra_menu_items = items;
        self
    }
}

impl Plugin for OsPlugin {
    fn build(&self, app: &mut App) {
        let app_name = self.app_name.clone();
        let extra_menu_items = self.extra_menu_items.clone();

        app.add_systems(Startup, move || {
            let mut menu = escher_os::menu::default_application_menu(&app_name);
            menu.extend(extra_menu_items.iter().cloned());
            if let Err(error) = escher_os::menu::set_application_menu(&menu) {
                tracing::warn!("Failed to set application menu: {error}");
            }
        });
    }
}
