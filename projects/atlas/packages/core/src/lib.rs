extern crate alloc;

//---
pub mod collections;

// Self-flagged by its own doc comment as "AI slop... shouldn't make it to the public repo,"
// enforced by a `#[cfg(not(debug_assertions))]` `compile_error!` inside the module itself —
// gated behind `dev` so a plain dependent (`escher-core`, which only ever wanted
// `collections::OrderedMap`) doesn't get dragged into that landmine on a `--release` build it
// never asked to compile this module for at all. `atlas-dev` (the only real consumer) opts in via
// this same feature on its own `atlas-core` dependency.
#[cfg(feature = "dev")]
pub mod tracing;
