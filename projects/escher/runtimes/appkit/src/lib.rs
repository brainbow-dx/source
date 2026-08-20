//! Renders `escher_core::Scaffold` trees as real, native AppKit views — see `surface`'s own doc
//! comment for the reconciliation contract. macOS only, same as `escher-webview`/parts of
//! `escher-os`; no equivalent backend exists yet for other platforms.
//!
//! The toolbar composition used to live here (`toolbar.rs`) — moved to `escher_chalk::toolbar`
//! once it turned out to have no actual dependency on anything AppKit-specific, the first entry in
//! that crate's shared component library. `tabs::tab_strip` stays here for now; see its own doc
//! comment (and `escher_chalk`'s) for why it isn't portable yet.
//!
//! The `bevy` feature (`bevy.rs`) is this crate's own optional Bevy-engine glue — a plugin exposing
//! neutral components/resources/messages, with every native AppKit call it makes staying inside
//! this crate. `escher-bevy` deliberately doesn't depend on this crate at all: a Bevy-integration
//! crate reaching into one specific native-UI backend would tie every consumer of it to that
//! backend, the same mistake this module itself exists to avoid one layer up.

#[cfg(target_os = "macos")]
mod action;
#[cfg(target_os = "macos")]
mod favicon;
mod hover;
#[cfg(target_os = "macos")]
pub mod surface;
#[cfg(target_os = "macos")]
mod views;

#[cfg(target_os = "macos")]
pub mod tabs;
#[cfg(target_os = "macos")]
pub mod shortcuts;

#[cfg(all(target_os = "macos", feature = "bevy"))]
pub mod bevy;

#[cfg(target_os = "macos")]
pub use surface::{AppKitSurface, FaviconImage, NativeEvent};

/// Fixed height, in points, every scene window reserves at its top for the toolbar — matches
/// `runtimes/os/src/macos/chrome.rs`'s old `CHROME_BAR_HEIGHT`, kept the same value for parity.
pub const TOOLBAR_HEIGHT: f64 = 44.0;
