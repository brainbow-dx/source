#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

//--
pub mod console;
pub mod error;

#[cfg(target_arch = "wasm32")]
pub mod surface;

// Target-agnostic: `ScaffoldDescription` + friends and the `Scaffold`-building logic don't touch
// `web_sys` — only the `mount_scaffold_from_json` wasm-bindgen entry point inside does, and
// that's cfg-gated within the module itself. Needed on native now too, for `ssg`'s SSG renderer.
pub mod description;

// Target-agnostic: the placeholder scaffold every new page starts from, shared by the wasm mount
// path (`surface::mount_scaffold`) and the native SSG fallback (`ssg::render_default_fragment`).
pub mod default_page;

// Native-only: renders a `Scaffold`/`ScaffoldDescription` to a static HTML string, for SSG.
// Doesn't touch `web_sys` at all (that's `surface`'s job, for the DOM/hydration path) — this
// produces plain markup, usable from `examples/serve.rs` without a browser.
#[cfg(not(target_arch = "wasm32"))]
pub mod ssg;
