//! The application menu bar (macOS) — standard, pre-wired roles (quit, hide, copy, paste, etc.)
//! route through AppKit's responder chain automatically with no target object needed; a custom
//! `MenuItem::Item` action needs a real Objective-C target object receiving the click, which
//! `macos::menu::MenuActionTarget` provides (same shape as `escher_appkit::action::ActionTarget`,
//! duplicated rather than depended on — `escher-appkit` depends on this crate, not the reverse).

use std::sync::Arc;

use crate::OsError;

/// A pre-wired standard menu action — the item's title, key equivalent, and click behavior are
/// all handled by AppKit/the responder chain; nothing here has to supply a callback.
#[derive(Debug, Clone, Copy)]
pub enum MenuRole {
    /// "Quit <app name>" — Cmd+Q.
    Quit,
    /// "Hide <app name>" — Cmd+H.
    Hide,
    /// "Hide Others" — Cmd+Option+H.
    HideOthers,
    /// "Show All".
    ShowAll,
    /// "Close Window" — Cmd+W.
    CloseWindow,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Clone)]
pub enum MenuItem {
    Role(MenuRole),
    /// A custom action item — `action` fires on click, real and wired (not the inert placeholder
    /// this used to be). `Arc<dyn Fn() + Send + Sync>`, not a plain `Rc`/`Box`: `MenuItem` has to
    /// stay `Send + Sync` since it lives in `escher_bevy::os::OsPlugin`, a real Bevy `Plugin`
    /// (which requires that bound); `Arc` also keeps a `MenuItem` tree cheaply `Clone`, matching
    /// `MenuRole`'s own `Copy`.
    Item { label: String, key_equivalent: String, action: Arc<dyn Fn() + Send + Sync> },
    Separator,
    Submenu { label: String, items: Vec<MenuItem> },
}

/// The conventional macOS app-menu shape: `<app name>` submenu (About/Hide/Quit) plus an Edit
/// submenu (Undo/Redo/Cut/Copy/Paste/Select All) — what every standard Mac app has by default.
pub fn default_application_menu(app_name: &str) -> Vec<MenuItem> {
    vec![
        MenuItem::Submenu {
            label: app_name.to_string(),
            items: vec![
                MenuItem::Role(MenuRole::Hide),
                MenuItem::Role(MenuRole::HideOthers),
                MenuItem::Role(MenuRole::ShowAll),
                MenuItem::Separator,
                MenuItem::Role(MenuRole::Quit),
            ],
        },
        MenuItem::Submenu {
            label: "Edit".to_string(),
            items: vec![
                MenuItem::Role(MenuRole::Undo),
                MenuItem::Role(MenuRole::Redo),
                MenuItem::Separator,
                MenuItem::Role(MenuRole::Cut),
                MenuItem::Role(MenuRole::Copy),
                MenuItem::Role(MenuRole::Paste),
                MenuItem::Separator,
                MenuItem::Role(MenuRole::SelectAll),
            ],
        },
    ]
}

/// Replaces the application's menu bar with `items`.
pub fn set_application_menu(items: &[MenuItem]) -> Result<(), OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::menu::set_application_menu(items)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = items;
        Err(OsError::Unsupported)
    }
}
