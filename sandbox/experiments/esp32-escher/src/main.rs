//! Stub for the "run Escher's own composition/styling core, backed by `bevy_ecs`, on a `no_std`
//! ESP32 target" experiment -- see `projects/escher/spec/.agents/proposals/esp32-no-std-game-dev.md`
//! for the full writeup. Not started as a real embedded program: this crate's only job right now is
//! the first, host-checkable question the proposal identifies -- does `escher-core` (with
//! `default-features = false`) and `bevy_ecs` (same) actually compile without `std`? That's a
//! language-level property (`#![no_std]` plus `alloc`), checkable on any host target; it does not
//! require an ESP32 toolchain, `esp-hal`/`esp-idf-hal`, or real hardware.
//!
//! As of this stub's own creation, the honest answer is **no, not yet**: `cargo check -p
//! escher-core --no-default-features --features macros` fails with 67 errors, all of two
//! mechanical kinds (see the proposal doc for the categorized breakdown) -- no `alloc`-crate
//! imports for `String`/`Vec`/`ToOwned`/`ToString`, and `derive_more`'s proc macros
//! (`Display`/`Deref`/`DerefMut`/`Index`/`IndexMut`/`IsVariant`) not enabled for a `no_std` build.
//! Fixing that is real, scoped, near-term work -- not attempted in this stub.

#![no_std]
#![no_main]

// Intentionally does not compile as a real program yet -- there's no `esp-hal`/`esp-idf-hal`
// runtime wired in (no `#[entry]`, no panic handler, no linker script), since the crate's own job
// right now is only the `escher-core`/`bevy_ecs` no_std compile check above, not a flashable
// binary. `cargo check` against this crate alone (without a real embedded target/runtime) won't
// fully succeed until that's added -- expected, not a bug in this stub.
