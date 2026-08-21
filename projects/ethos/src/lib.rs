// `oops::Error`'s derive macro expands to `alloc::format!(...)` regardless of whether the crate
// is `no_std` — needs `alloc` explicitly in scope even here, where std is otherwise available.
extern crate alloc;

pub mod error;
pub mod exec;
pub mod log;
