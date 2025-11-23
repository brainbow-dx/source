extern crate slate_core;
pub use crate::slate_core::*;

pub extern crate slate_macros as macros;
pub use crate::macros::*;

//--
#[cfg(all(feature="terminal", target_arch="wasm32"))]
compile_error!("Terminal is unsupported in wasm (for now).");

#[cfg(all(feature="terminal", not(target_arch="wasm32")))]
pub extern crate slate_terminal as terminal;

//--
pub extern crate slate_bevy as bevy;

//--
pub mod prelude {
    pub use slate_core::prelude::*;
}
