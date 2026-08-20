extern crate alloc;

// Built on `atlas_core::tracing`, self-flagged by its own doc comment as "AI slop... shouldn't
// make it to the public repo," enforced by a `compile_error!` on any `--release` build — gated
// here too so a plain dependent of this crate (`atlas`'s own facade re-export, and anything
// depending on *that*, like `apps/anvil`) doesn't get dragged into that landmine without ever
// actually using this module.
#[cfg(feature = "dev")]
pub mod tracing;