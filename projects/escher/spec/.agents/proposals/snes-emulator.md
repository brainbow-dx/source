# SNES emulator (vision, thought experiment)

Status: vision only. A stub experiment exists at `sandbox/experiments/snes-emu/` (a bare Bevy
window, no emulator core wired in yet) — nothing else here is started. Written up per direct user
request after a "simple thought experiment" conversation, so the reasoning isn't lost.

Two genuinely different-sized ideas, not one project with two phases of the same thing:

## A: a software SNES core, backed by Escher's UI/scene/audio stack

Straightforward to scope, and fits cleanly onto what this session already proved out (mario's
`bevy_audio` wiring, `bevy_gilrs` gamepad input, a framebuffer-style render loop). Three
well-separated hardware subsystems to emulate:

- **CPU (65816)**: a 16-bit extension of the 6502. Well documented, abundant reference
  implementations to check timing/behavior against. A hobbyist-grade core here is a few weeks of
  focused work, not a research problem.
- **APU (SPC700 + DSP)**: the SNES's sound runs on a genuinely separate embedded CPU + DSP,
  communicating with the main CPU over a small, quirky port. Also well-isolated and
  well-documented; a few weeks.
- **PPU (Picture Processing Unit)**: the hard part. Background layers, sprites, Mode 7 affine
  transforms, HDMA, color math, mid-scanline effects. A "plays most simple games reasonably" PPU
  is weeks-to-months; a "matches bsnes/higan-level accuracy across the whole library" PPU is the
  multi-year project those projects actually were. Scope to the former for a first cut.

**Existing Rust prior art, real not assumed** — checked via web search + a fetch of the repo
itself, not recalled from training data: [`nat-rix/rsnes`](https://github.com/nat-rix/rsnes) is a
real, structured `rsnes` (backend library) + `rsnes-emulator` (frontend) workspace, MIT licensed,
with a sample `winit`/`wgpu` frontend. Real caveats, not just "it exists": the maintainer's own
README says the library API is "neither tested nor documented (well)"; it's git-only (not on
crates.io); the frontend is only tested on Linux/X11; SPC700/DSP audio is partial (echo/noise
effects listed as future work); real gamepad support isn't done yet (keyboard bindings only,
blocked on a `winit` issue). Worth reading as a reference/inspiration for timing and structure,
not worth depending on directly as-is.

**Escher integration shape**, once a core (borrowed, forked, or written fresh) exists:
- Framebuffer → texture upload once per emulated frame, the same shape any Bevy 2D game already
  uses for a dynamically-updated sprite.
- Audio → `bevy_audio`, the same mechanism `runtimes/bevy/examples/mario`'s sfx already proved out
  this session (`escher-bevy`'s own `audio` Cargo feature, off by default — this experiment would
  need to opt in the same way).
- Input → `bevy_gilrs`, already proven end-to-end in `mario`'s own gamepad handling.

None of this needs new Escher/Ethos infrastructure. It's an application of what already exists,
not a new capability.

## B (the real stretch): generating machine code that runs on real SNES hardware

A fundamentally different kind of problem: not "emulate the SNES," but "be a compiler backend +
homebrew SDK targeting the 65816." Real prior art exists here too — `cc65`, `WLA-DX`, `PVSnesLib`
— and those took years, are still maintained, and are the honest baseline to compare any new
effort against, not a strawman.

What it would actually take:
- **A 65816 code generator.** There's no LLVM backend for this ISA. The tractable path (what most
  niche retro toolchains actually do) is emitting textual 65816 assembly from an IR and shelling
  out to an existing assembler (`WLA-DX`, `ca65`), not writing a true codegen backend against a
  register allocator from scratch — the 65816 has almost no general-purpose registers (`A`/`X`/`Y`
  plus direct-page addressing tricks), a genuinely awkward target for a conventional backend.
- **A minimal runtime**: hand-written startup code (reset vector, PPU register setup, DMA-driven
  graphics upload, bootstrapping a tiny program onto the SPC700 to drive audio — itself a whole
  separate, quirky subsystem), all within a very small address space (ROM banks, ~128KB WRAM).
- **A real rethink of what "Ethos codegen from scaffolds" even means here** — this is the load-
  bearing caveat, not a footnote. `escher-core`'s `Scaffold`/flexbox-like composition model
  assumes arbitrary layout and per-pixel/vector rendering. None of that exists on real SNES
  hardware: rendering is fixed tile-based background layers, a hard sprite-count/size budget, and
  palette-constrained VRAM. A `Scaffold` → SNES pass isn't a new backend under the existing model,
  it's a different, much more constrained scene description built for tile/sprite/palette
  primitives specifically. Skipping this rethink and assuming the existing UI composition model
  just needs "a SNES target" would produce something that compiles but doesn't correspond to
  anything the hardware can actually do.

**Recommendation if this ever gets pursued for real**: lean on the existing 65816 toolchain
(`cc65`/`WLA-DX`/`PVSnesLib`) rather than reinventing a compiler backend — Ethos's real value-add
in that world is a code-generation pass from some SNES-shaped scene description down to calls into
that existing SDK, not a from-scratch compiler. That's still a substantial, multi-month-plus
project, just a much smaller one than "write a 65816 compiler backend from nothing."

## Recommendation

Part A is a real, scoped, achievable project if the actual goal is "SNES-feeling games running
today, using tools already in this stack." Part B is worth revisiting only if the actual goal
becomes "produce a real cartridge," and even then the honest starting point is the existing
homebrew toolchain, not a bespoke backend — and the `Scaffold` mismatch needs solving first, not
assumed away.
