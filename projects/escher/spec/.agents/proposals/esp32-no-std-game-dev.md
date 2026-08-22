# Escher + Bevy ECS on ESP32 (no_std), for real hardware game dev

Status: exploration task, not started. Written up per direct user request, to slot into the
roadmap and keep momentum for a "look into this soon" item — not vision-only in the SNES
proposal's sense (see "Relationship to the SNES proposal" below): the underlying platform
(`bevy_ecs` no_std on ESP32) is already real and demonstrated by others today, and a first,
concrete, host-checkable question about Escher's own readiness has already been answered here,
not left as a guess.

## The goal, stated directly

Use Escher (its `Scaffold` composition/styling core, not a from-scratch UI layer) to assist
building **real games that run natively on ESP32-class hardware** — audio, OLED/small displays,
buttons/analog input — not an emulator, not a desktop dev-time preview. Per the user's own
framing: "our current experiments work by generating a lot of code and shipping a very small rust
core, but none of that work with escher yet" — i.e. there's prior art in this codebase's own
orbit for the "generate code, ship a tiny native core" shape this would need, just not yet wired
to Escher's own composition/styling layer. (Referenced as stated by the user; this proposal
doesn't independently trace that prior experiment's own code — worth linking explicitly once
picked up for real.)

## What already works, real and demonstrated — checked via web search + a fetch, not assumed

**`bevy_ecs` genuinely runs on ESP32 today, no emulation involved.** As of **Bevy 0.16** (2025),
the core `bevy` crate and many subcrates gained real `no_std` support, backed by a new
[`bevy_platform`](https://crates.io/crates/bevy_platform) crate providing `std`-alternative
primitives. Opt in via `bevy = { version = "0.16", default-features = false }`.
([Bevy 0.16 release notes](https://bevy.org/news/bevy-0-16/))

Espressif's own developer blog (April 2025) demonstrates this concretely, not theoretically: two
working apps — a Conway's Game of Life and a "Spooky Maze" game with player movement, NPCs, and
collectibles — running Bevy's ECS on an **ESP32-S3-BOX-3** (ESP32-S3 + PSRAM + touch display) and
an **ESP32-C3**, using real `no_std` Rust.
([Espressif Developer Blog](https://developer.espressif.com/blog/2025/04/bevy-ecs-on-esp32-with-rust-no-std/))

**What's explicitly still missing, straight from Bevy's own release notes**: "Rendering, audio,
and assets are notable missing APIs" in `no_std` mode. The Espressif demos work around this by
driving the display themselves via the `embedded-graphics`/`mipidsi` crates, not `bevy_render` —
there is no shared, reusable "Bevy ECS + small display" rendering crate in the ecosystem today;
each project hand-rolls it. **This is a real gap Escher might be able to fill for the broader
`bevy_ecs`-on-embedded community**, not just for this repo's own use — see "A possible community
angle" below.

## Escher's own no_std readiness — checked for real, right now, not assumed

`escher-core` (the `Scaffold` composition/styling crate) already declares `no_std` intent:
`src/lib.rs` opens with `#![cfg_attr(not(feature = "std"), no_std)]`, and its `Cargo.toml` has a
real `std` feature (on by default) separate from `macros`/`dev`/`verbose`/`profiling`. This is a
better starting position than a from-scratch effort — but it's aspirational, not yet exercised:

```sh
cargo check -p escher-core --no-default-features --features macros
```

fails with **67 errors**, all of exactly two mechanical kinds (full breakdown in
`sandbox/experiments/esp32-escher/README.md`, which reproduces this check):

- Missing `alloc`-crate imports (`String`/`Vec`/`&str::to_owned`/`&str::to_string` used as if from
  `std`'s prelude, ~19 errors).
- `derive_more`'s proc macros (`Display`/`Deref`/`DerefMut`/`Index`/`IndexMut`/`IsVariant`) not
  resolving — `derive_more` itself supports `no_std`, so this reads as a missing feature flag in
  `escher-core`'s own `Cargo.toml`, not a fundamental incompatibility (~44 errors).

**No sign of a deeper architectural blocker from this quick pass.** Both error classes are the
kind a real no_std migration fixes mechanically (add `extern crate alloc;` imports, wire up
`derive_more`'s no_std feature), not a redesign. That's genuinely encouraging for how tractable
this is — but it is real, unstarted work, not "basically already works."

Not yet checked at all: `escher-terminal`, `escher-bevy`, `escher-styleguide`, or anything past
`escher-core` itself — the dependency graph gets wider fast (`escher-bevy` alone pulls in the
full `bevy` crate with `2d`/`ui` features, `ratatui`/`crossterm` for the terminal feature, `tokio`
on native targets — none of that audited for no_std compatibility yet).

## Near-term ask (the actual "look into this soon" scope)

Roughly in order, each a real, boundable step:

1. **Fix `escher-core`'s no_std compile errors** — the two mechanical classes above. Small, real,
   scoped; the first concrete PR-sized piece of this whole effort.
2. **Verify `bevy_ecs` (no_std) + fixed `escher-core` (no_std) actually compose** — do they
   conflict on allocator setup, panic handler expectations, anything else that only shows up once
   both are in the same crate graph? `sandbox/experiments/esp32-escher/` is the stub for this,
   currently blocked on step 1.
3. **A minimal `Scaffold` → `embedded-graphics` rendering path** — genuinely new work, since
   Bevy's own no_std mode ships without `bevy_render` (confirmed above) and Escher's own
   `Scaffold` rendering today assumes a `bevy_render`-backed 2D/UI pipeline. Scope small first: can
   a `Scaffold`'s box-model/color output drive `embedded-graphics`'s primitive drawing calls
   (rectangles, text) onto a monochrome/small-color-depth OLED framebuffer? Full flexbox-style
   layout is not required for a first cut — most embedded displays are small enough that simple,
   mostly-static layouts cover a lot of real use.
4. **A real flashable target** — `esp-hal` (bare-metal `no_std`) vs. `esp-idf-hal` (the `std`-on-
   FreeRTOS path Espressif's own demos use, easier bring-up, slightly heavier). Espressif's demos
   used `esp-idf-hal`-adjacent tooling; worth checking whether that's still the pragmatic starting
   choice or whether bare-metal `esp-hal` is now smoother, since this space moves fast.
5. Only once 1-4 are real: audio (ESP32 has real DAC/I2S options, a much smaller lift than
   `bevy_audio`'s desktop-oriented `cpal`/`rodio` stack) and input (buttons/analog, not a gamepad
   API at all on this class of hardware).

Stub experiment: `sandbox/experiments/esp32-escher/` (step 2's placeholder, currently just proves
step 1 hasn't happened yet — see its own README for the exact, reproduced error breakdown).

## A possible community angle, per the user's own "funny, I think we have what we need" reaction

Bevy's own no_std mode explicitly ships without rendering (confirmed above), and there's no
shared "Bevy ECS + small embedded display" crate in the ecosystem — every `bevy_ecs`-on-ESP32
project (Espressif's own demos included) hand-rolls its own `embedded-graphics` integration from
scratch. If `escher-core`'s `Scaffold` becomes genuinely no_std-capable (step 1-3 above), a
minimal, standalone "`Scaffold` → `embedded-graphics`" rendering crate could plausibly be useful
to the wider `bevy_ecs`-on-embedded community as its own small, focused open-source piece — not
gated on the rest of this proposal's own game-dev goal, and not something to build *for* that
reason first, but worth keeping in mind as a natural byproduct once step 3 exists for real. Not
scoped further here; a real "should we publish this separately" decision for once it exists.

## Relationship to the SNES proposal (`proposals/snes-emulator.md`)

Considered combining these into one document, per direct request — deliberately did not, and
here's the honest reasoning so it can be overridden: the two problems are genuinely different
sizes and shapes, not two phases of the same project.

- **ESP32 already has a real, working, modern Rust target** — full LLVM backend, real (if young)
  `no_std` support in the Bevy ecosystem, demonstrated by a third party on real hardware. The work
  here is "wire up and extend an existing path," bounded and mechanical at every step identified
  above.
- **The SNES's real-hardware path (that proposal's own "Part B") is a from-scratch compiler-
  backend problem** — there is no LLVM target for the 65816 at all; the honest path leans on
  existing 65816-specific toolchains (`cc65`/`WLA-DX`) rather than writing a new codegen backend,
  and that's still explicitly scoped as a "multi-month-plus" stretch goal in that document, not
  near-term work.

Forcing these into one proposal risks implying they're comparably-sized problems, which they
aren't — one is "extend a working modern toolchain," the other is "build a compiler backend for a
1990s ISA with none today." **The real, honest shared thread** worth naming (not merging) is
architectural: both eventually need Escher's `Scaffold` composition model to target genuinely
constrained rendering primitives instead of arbitrary flexbox-style layout — tile/sprite/palette
layers for SNES-class hardware, a small framebuffer via `embedded-graphics` for ESP32-class
hardware. A generic "`Scaffold` → constrained-hardware-rendering" abstraction, parameterized by
target primitives, is a real, worth-revisiting idea *once both sides have a concrete
implementation to generalize from* — not before, and not as a premature shared crate designed
against zero real examples. Cross-referenced here and worth adding a matching pointer to the SNES
proposal once someone actually starts step 3 above.

## Recommendation

Real, scoped, near-term work — not a "vision only" entry in the same sense as the SNES proposal.
Step 1 (`escher-core`'s no_std compile fixes) is a concrete, boundable, low-risk piece worth
picking up soon on its own merits, independent of how far the rest of this goes.
