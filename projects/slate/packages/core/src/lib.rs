#![cfg_attr(not(feature = "std"), no_std)]
#![feature(allocator_api)]
#![feature(const_type_id)]
#![feature(type_alias_impl_trait)]
#![feature(unboxed_closures)]

extern crate alloc;

//---
pub extern crate slate_macros as macros;

pub mod context;
pub mod element;
pub mod event;
pub mod log;
pub mod scaffold;
pub mod style;
pub mod surface;
pub mod prelude {
    pub use crate::style::prelude::*;
}
