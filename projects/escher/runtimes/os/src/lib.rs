//! OS-level integration that isn't specific to any one engine/runtime — native dialogs, the
//! application menu bar, clipboard access. macOS only for now (`objc2`/AppKit); other platforms
//! compile but every call returns [`OsError::Unsupported`]. Consumers (e.g. `escher-bevy`) treat
//! this the same way they treat `escher-webview`: a crate they call into, not something they own.
//!
//! The chrome bar (browser back/forward/address toolbar) used to live here, hosted via a compiled
//! Swift/SwiftUI dylib (`build.rs`'s old `swiftc` step) — moved to `escher-appkit` and rebuilt as
//! a real `escher_core::Scaffold` composition, rendered through plain `objc2`/AppKit like the rest
//! of this crate, no Swift toolchain involved. See `escher_appkit::chrome`.
//!
//! OS packaging (building a `.app` bundle, code signing, installer generation) is a distinct,
//! build-time concern from this crate's runtime API and isn't implemented here yet.

#[cfg(target_os = "macos")]
mod macos;

pub mod activation;
pub mod clipboard;
pub mod dialog;
pub mod menu;
pub mod process;
pub mod sound;
pub mod terminal;

#[derive(Debug)]
pub enum OsError {
    /// No backend exists for this platform yet (only macOS today).
    Unsupported,
    /// AppKit (and most native UI toolkits) requires calls to happen on the main thread.
    NotOnMainThread,
    /// A named resource (e.g. a system sound name) doesn't exist on this system.
    NotFound(String),
    /// The call reached the OS but the OS itself reported a real failure (a failed process spawn,
    /// a nonzero exit) — distinct from `Unsupported`, which means no attempt was even made.
    Failed(String),
}

impl std::fmt::Display for OsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsError::Unsupported => write!(f, "unsupported on this platform"),
            OsError::NotOnMainThread => write!(f, "must be called from the main thread"),
            OsError::NotFound(name) => write!(f, "{name:?} not found"),
            OsError::Failed(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for OsError {}
