# SNES emulator experiment (stub)

Not started — this is a placeholder window, nothing emulator-related is wired in yet. See
`projects/escher/spec/.agents/proposals/snes-emulator.md` for the full writeup: what a real core
would take, what already exists in the Rust ecosystem (checked for real, not assumed), and a much
bigger "generate real SNES machine code" stretch goal this experiment is deliberately *not*
attempting.

Run it:

```
cargo run
```

Standalone (its own `[workspace]`), not part of Escher's — this stays decoupled from the real
workspace while it's still speculative. Replace `spawn_placeholder_screen`/
`pulse_placeholder_screen` in `src/main.rs` with a real emulator core's framebuffer output to
actually start this.
