extern crate slate_core;
pub use crate::slate_core::*;

pub extern crate slate_macros as macros;
pub use crate::macros::*;

pub extern crate slate_terminal as terminal;

pub extern crate slate_bevy as bevy;

pub mod prelude {
    pub use slate_core::prelude::*;
}