# Anvil command host API (proposed, not started)

Flagged in PR review: `commands/quit.js` is a one-line script that returns a magic sentinel string (`QUIT_SENTINEL` in `main.rs`) which Rust then greps for in a command's output to decide to exit. `commands/clear.js` follows the identical pattern with its own `CLEAR_SENTINEL`. The reviewer's point: a command script should be able to *do* the thing directly — call a real host-exposed function — rather than communicate intent back through a string Rust has to interpret after the fact. Right now the split between Rust and the scripting layer is "Rust bootstraps and runs the script, then still has to understand what happened"; the goal is commands loaded, registered, and run entirely within their own script files, with no Rust-side interpretation of their output beyond "show this text."

## Why this isn't a quick fix

`ethos-deno` already has the real mechanism for this — `packages/deno/src/runtime.rs`'s `ethos_sdk` extension registers `op_send_host_log` via `deno_core::extension!`/`op2`, a working example of a JS-callable host function. Adding an `op_anvil_quit` (or a more general `op_anvil_action`) the same way is mechanically straightforward. The part that isn't: threading the result back to Anvil's own `AppExit` on the Bevy main loop. The op fires on whatever thread/executor is running the Deno runtime for that command invocation, not the Bevy `Update` schedule — today's sentinel-string return value already crosses that boundary safely because `spawn_js_command`'s result handling runs back on the app's own state. A host op needs the same safe handoff (a channel, an `Arc<RwLock<_>>` flag, whatever `AppState` already uses elsewhere) designed deliberately, not improvised mid-command.

## Sketch

- A small `AnvilHostApi` extension (mirroring `ethos_sdk`'s shape) registered wherever Anvil constructs its own Deno runtime for command execution (`process::run_deno_command` or nearby).
- Start with the two known cases: `op_anvil_quit()` and `op_anvil_clear_transcript()`, replacing `QUIT_SENTINEL`/`CLEAR_SENTINEL` entirely once wired.
- Whatever channel/flag mechanism carries the op's effect back to the main app state should be generic enough that a third command (a future one) doesn't need a new plumbing pattern, just a new op + a new `AppState` field to react to.
- `commands/relay-console`'s own case is related but separate: `/relay-console` is currently a Rust-hardcoded `SlashCommand` (not a discovered script) whose Rust handler shells out to `scripts/serve-relay-console.ts`/`scripts/open-page.js` directly. Whether it should become a real discovered command built on this same host API, or stay Rust-native since it manages a long-lived subprocess (not a fire-and-forget script), is a separate design question worth resolving alongside this one — it's the same underlying "what belongs in Rust vs. the script layer" question, just with a stateful process attached instead of a one-shot action.

## Status

Proposed only. `quit.js`/`clear.js` still use the sentinel-string pattern; not blocking, but real commands should migrate to the host API once it exists rather than gaining more sentinel strings in the meantime.
