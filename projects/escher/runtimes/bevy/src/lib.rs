pub mod config;
pub mod plugin;
pub mod reticle;
pub mod surface;
pub mod time;

#[cfg(feature = "terminal")]
pub mod terminal;

// `escher-webview` (the crate this module talks to) already degrades gracefully to
// `Err(UnsupportedWindowHandle)` on platforms without a backend, so this module doesn't need its
// own `#[cfg(target_os = "macos")]` gate — it compiles everywhere, same as any other module here.
pub mod webview;

pub mod os;

pub mod log;

// `src/legacy/` (window.rs, webview.rs, input.rs, provider.rs) is reference-only source ported
// from a pre-refactor `slate` prototype — deliberately NOT declared as a module here. See
// `src/legacy/README.md` for why each file needs real rework before it can compile.

pub use config::EscherBevyConfig;
pub use plugin::EscherBevyPlugin;
