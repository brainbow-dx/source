extern crate alloc;

//--
pub mod app;

pub mod event;

pub mod surface;

pub mod error;

pub mod text_wrap;

pub mod tracing_bridge;

// `spawn_input_watcher` (plain crossterm polling) works on any target; `spawn_signal_watcher`/
// `reraise_signal` are individually `#[cfg(unix)]` inside the module itself, not gated here —
// this crate already has a real Windows leg (`[target.'cfg(target_os="windows")'.dependencies]`
// in its own `Cargo.toml`), so hiding the whole module would take the input watcher down with it.
pub mod watch;

//--
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    //..
}
