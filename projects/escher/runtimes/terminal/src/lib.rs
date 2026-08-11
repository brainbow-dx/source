#![feature(unboxed_closures)]

extern crate alloc;

//--
pub mod app;

pub mod event;

pub mod surface;

pub mod error;

//--
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    //..
}
