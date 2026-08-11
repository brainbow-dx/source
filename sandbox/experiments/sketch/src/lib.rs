#![feature(allocator_api)]
#![feature(unboxed_closures)]

extern crate alloc;

//--
pub mod error;

//--
include!("lib.c.rs");