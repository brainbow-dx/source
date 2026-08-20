#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

//---
pub extern crate escher_macros as macros;

pub mod animate;
pub mod draw;
pub mod surface;
pub mod scaffold;
pub mod element;
pub mod content;
pub mod style;
pub mod event;
pub mod log;

pub mod prelude {
    pub use crate::style::prelude::*;
    pub use crate::content::prelude::*;
    pub use crate::surface::prelude::*;
    pub use crate::scaffold::prelude::*;
    pub use crate::event::prelude::*;
    pub use crate::element::prelude::*;
    // pub use crate::draw::prelude::*; // `draw::DrawReport` is still a placeholder type; no prelude yet.
}
