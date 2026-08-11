extern crate escher_core;
pub use crate::escher_core::*;

pub extern crate escher_macros as macros;
pub use crate::macros::*;

//--
#[cfg(all(feature="terminal", target_arch="wasm32"))]
compile_error!("Terminal is unsupported in wasm (for now).");

#[cfg(all(feature="terminal", not(target_arch="wasm32")))]
pub extern crate escher_terminal as terminal;

//--
pub extern crate escher_bevy as bevy;

//--
pub mod prelude {
    pub use escher_core::prelude::*;
}
