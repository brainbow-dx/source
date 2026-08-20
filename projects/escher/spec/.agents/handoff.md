# Handoff — 2026-08-20 (late) — Mario example's self-terminating-within-150ms bug found and live-verified fixed; everything else from earlier today is unverified going into tomorrow

Overwritten each time there's a natural stopping point. Current entry only; don't append history here — see `changelog.md` for the full trail, or `spec/ROADMAP.md` for the standing cross-session milestone tracker.

## Resolved this round, live-verified

The user reported that nothing claimed fixed earlier today actually turned out to be fixed, and asked for one last real attempt at the `mario` example specifically (`runtimes/bevy/examples/mario`) before calling it a night — they want to install it on another machine tomorrow. Checked it directly rather than trusting the earlier claims: built it fresh in an isolated `CARGO_TARGET_DIR`, then ran the actual binary through a real pty (not just `cargo check`) and read its own trace log.

Found a real bug: the process was self-terminating within ~150ms of every launch, unconditionally, regardless of gamepad/relay/network state. `bevy_window::system` logged `"No windows are open, exiting"` — Bevy's default `ExitCondition::OnAllClosed` exit system was firing because `main.rs`'s `EscherBevyConfig` sets `with_spawn_primary_window(false)` (mario is terminal-only; only `scene.rs`'s `B`-toggle ever opens a real window) without also setting `with_exit_condition(ExitCondition::DontExit)` — a window count of zero also satisfies "all closed." This is the exact seam `EscherBevyConfig::exit_condition`'s own doc comment already describes; `apps/anvil` already uses `DontExit` for the identical reason. Fixed with one added `.with_exit_condition(ExitCondition::DontExit)` call in `main.rs`.

Live-verified both directions via pty, not just re-read code: before the fix, the process reliably exits (status 0) around 150ms after startup with zero input. After the fix, it stays alive indefinitely with no input. Also re-checked that quitting still works after the config change — pressing Escape still exits cleanly (status 0, ~50ms) via the terminal's own `TerminalAction::Exit` path, which is independent of the window exit condition. `cargo build -p escher-bevy --example mario` is clean.

## Not touched this round — everything else is unverified going into tomorrow

The user explicitly said they're done for tonight and will look at everything else themselves tomorrow. Nothing else from earlier today's round (`/relay-console`, trace-noise, logs location, window-raise, docker-autostart, the visual design pass) was re-touched or re-verified in this round — see this file's prior entry, preserved below only because the user's own plan is to re-check it personally, not because it's trusted as fixed.

Given the user's own words ("once again I've confirmed that none of the things we've been working on that you've claimed are fixed are actually fixed"), the standing methodology note from the prior entry needs restating stronger: don't report anything as fixed here without having actually run the real binary and observed the real behavior first-hand, in the same way this round's mario fix was checked. Re-running a fresh instance and reading `cargo check` output is not verification of runtime behavior.
