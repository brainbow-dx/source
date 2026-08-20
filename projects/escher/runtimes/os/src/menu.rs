//! The application menu bar (macOS) — built from standard, pre-wired roles (quit, hide, copy,
//! paste, etc.) rather than arbitrary custom actions. Standard AppKit action selectors
//! (`terminate:`, `copy:`, ...) route through the responder chain automatically with no target
//! object needed — that's *why* only roles are supported today: a custom click-handler (an
//! arbitrary Rust closure firing on click) needs a real Objective-C target object receiving the
//! action selector, which is real, nontrivial work not attempted in this pass. Plain `Item`s are
//! visible but inert until that lands.

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

#[derive(Debug, Clone)]
pub enum MenuItem {
    Role(MenuRole),
    /// A plain, currently-inert item (see this module's doc comment).
    Item { label: String, key_equivalent: String },
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
