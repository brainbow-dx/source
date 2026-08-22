# esp32-escher (stub)

Not started as a real embedded program — this is a placeholder for "run Escher's own
`Scaffold` composition/styling core, backed by `bevy_ecs`, on a `no_std` ESP32 target." See
`projects/escher/spec/.agents/proposals/esp32-no-std-game-dev.md` for the full writeup: what
already works in the wider ecosystem (real, demonstrated `bevy_ecs` no_std support on ESP32 as of
Bevy 0.16), what Escher's own no_std readiness actually looks like today (checked for real, not
assumed — see below), and the shape of the near-term work.

## The one thing this stub actually checks

`Cargo.toml` depends on `bevy_ecs` and `escher-core`, both `default-features = false` — the
`no_std` posture both crates already declare intent for (`escher-core/src/lib.rs`'s own
`#![cfg_attr(not(feature = "std"), no_std)]`). This is a **language-level** question
(`#![no_std]` + `alloc`), not a hardware one — checkable on any host target, no ESP32 toolchain,
`esp-hal`, or real board required.

Run it:

```sh
cargo check -p escher-core --no-default-features --features macros
```

(from the real `projects/escher` workspace, not this standalone one — `escher-core`'s own
`Cargo.lock`/dependency resolution needs to be the real one, not this experiment's minimal
sandbox lockfile).

**As of this stub's own creation, the honest answer is no, not yet.** 67 errors, all of two
mechanical kinds:

- Missing `alloc`-crate imports: `String`/`Vec`/`&str::to_owned`/`&str::to_string` are used as if
  from `std`'s prelude, but nothing imports them from `alloc` (`extern crate alloc;
  use alloc::{string::String, vec::Vec, ...};`) for the `no_std` path.
- `derive_more`'s proc macros (`Display`, `Deref`, `DerefMut`, `Index`, `IndexMut`, `IsVariant`)
  aren't resolving — `derive_more` itself supports `no_std`, so this reads as a feature-flag
  wiring gap in `escher-core`'s own `Cargo.toml` (not requesting whatever feature enables its
  macros without `std`), not a fundamental incompatibility.

No sign, from this quick check, of a deeper architectural blocker — both error classes are the
kind of thing a real no_std migration pass fixes mechanically, not a redesign. That pass itself
is real, scoped, near-term work, not attempted here.

## What's still not started

- Actually linking against `esp-hal` (bare-metal) or `esp-idf-hal` (the `std`-on-FreeRTOS path
  Espressif's own real demos use) and getting a flashable binary — this stub has no `#[entry]`,
  panic handler, or linker script.
- A `Scaffold` → `embedded-graphics` rendering backend (Bevy's own `no_std` mode ships without
  `bevy_render` — see the proposal doc — so this is genuinely new work, not reusing an existing
  Bevy rendering path).
- Audio, input, and everything else the proposal's "near-term ask" section scopes.

Standalone (its own `[workspace]`), not part of Escher's real one — same reasoning as the sibling
`sandbox/experiments/snes-emu`: stays decoupled from the real workspace while still speculative.
