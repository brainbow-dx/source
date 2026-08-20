# Proposal: restore the terminal on SIGTERM/SIGHUP/SIGINT, not just in-process panics

Status: **implemented (2026-08-15, human-insisted) — with one fix beyond this proposal's original design.** The
flag-based `signal_hook::flag::register_usize` approach below was implemented directly in `apps/anvil` (not
`runtimes/terminal/src/app.rs` — this app doesn't go through `TerminalApp::run` at all, see `TerminalHandle::
signal_flag` in `apps/anvil/src/main.rs`). It worked, but far too slowly: `WinitSettings::desktop_app()` only
ticks Bevy's `Update` schedule (where the flag is checked) reactively, so a real `SIGTERM` took **~13 seconds**
to actually exit — measured live, not assumed. Fixed by switching to `signal_hook::iterator::Signals` on a real
background thread (mirroring `spawn_input_watcher`'s existing shape), which calls `EventLoopProxy::send_event
(WakeUp)` the moment a signal arrives instead of only setting an atomic — forces an immediate tick. Re-measured:
**~125ms**. See `changelog.md`'s matching entry.

## The bug

`TerminalApp::run` (`runtimes/terminal/src/app.rs`) enables raw mode, the alternate screen, and mouse capture, and
installs a panic hook that calls `restore_terminal()` before delegating to the previous hook. That covers in-process
Rust panics correctly. It does **not** cover the process being killed by an external signal — there's no signal
handler registered at all today.

Reported symptom: "the assistant sometimes dies not very gracefully (usually killed by another process). When that
happens terminal output in the remaining shell is still taking user input so spams the std feed with garbage."

Root cause: `kill <pid>` / `pkill -f <pattern>` with no `-9`/`-KILL` sends `SIGTERM` by default. With no handler,
the OS terminates the process immediately — raw mode and mouse capture are still enabled on the real terminal, and
the parent shell that regains control starts reading raw escape sequences / individual unbuffered keystrokes. That's
the "spams the feed with garbage" symptom. This literally happened earlier in this session: a subagent's cleanup ran
`pkill -f "target/debug/examples/assistant"`, which is SIGTERM, not SIGKILL.

Note: `SIGKILL` (`kill -9`) genuinely cannot be caught by any process at the OS level — out of scope, not fixable.
The realistic, fixable case is plain `SIGTERM` (the default for `kill`/`pkill`), plus `SIGHUP`/`SIGINT` for the same
reason. `SIGINT` here mainly covers being killed via terminal-close/hangup-adjacent paths — not in-app Ctrl+C, which
crossterm's own event loop already reads as a literal byte under raw mode and which `TerminalApp` already handles
via its own `TerminalAction::Exit`.

## Proposed design

Use the [`signal-hook`](https://docs.rs/signal-hook) crate's flag-based API — no `unsafe`, async-signal-safe by
construction, and no async runtime required (`escher-terminal`'s core crate has no `tokio` dependency and shouldn't
gain one just for this; the render loop in `TerminalApp::run` is fully synchronous).

### 1. Dependency

`projects/escher/Cargo.toml`, `[workspace.dependencies]` (alongside the other terminal-adjacent pins):

```toml
signal-hook = { version = "0.3", default-features = false }
```

`runtimes/terminal/Cargo.toml`, gated the same way the existing Windows-only `crossterm` feature block is gated
(this crate has no `cfg(unix)` block yet, so this introduces the first one, mirroring the workspace root's existing
`[target.'cfg(target_os="windows")'.dependencies]` pattern):

```toml
[target.'cfg(unix)'.dependencies]
signal-hook = { workspace = true }
```

### 2. Registration, right after the panic hook is installed in `TerminalApp::run`

```rust
#[cfg(unix)]
let signal_flag = {
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    let flag = Arc::new(AtomicUsize::new(0));
    for sig in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGHUP, signal_hook::consts::SIGINT] {
        // register_usize (not the plain bool `register`) so the loop below can tell which
        // signal fired, needed later to re-raise the *same* signal via emulate_default_handler
        // rather than always terminating as if it were SIGTERM.
        signal_hook::flag::register_usize(sig, Arc::clone(&flag), sig as usize)?;
    }
    flag
};
```

`register_usize` writes the raw signal number into the shared `AtomicUsize` the moment the signal arrives — the
handler itself does nothing beyond that atomic store, which is what makes it async-signal-safe without any `unsafe`
on our side.

### 3. Checked once per render-loop iteration, alongside the existing `draw_surface_fn` call

```rust
loop {
    #[cfg(unix)]
    {
        let sig = signal_flag.load(std::sync::atomic::Ordering::Relaxed);
        if sig != 0 {
            #[cfg(feature = "dev")]
            tracing::debug!("Received signal {sig}, restoring terminal and exiting ..");
            break;
        }
    }

    match draw_surface_fn(&mut surface) {
        // ... unchanged
    }
}
```

Falling through to the existing post-loop cleanup (`DisableFocusChange`/`DisableMouseCapture`/
`DisableBracketedPaste`/`DisableBlinking`, then `restore_terminal()`) requires no changes — it already runs
unconditionally after the loop regardless of which `break` was taken.

### 4. Actually terminating after cleanup

The caller (or whatever sent the signal) expects the process to actually go away, not linger — so after
`restore_terminal()` runs, re-raise the same signal with its default disposition via `signal-hook`'s own documented
"clean up then really exit" pattern:

```rust
#[cfg(unix)]
{
    let sig = signal_flag.load(std::sync::atomic::Ordering::Relaxed);
    if sig != 0 {
        // Restores the default handler for `sig` and re-raises it, so the process exits with
        // the conventional 128+signal status instead of silently returning `Ok(())` as if this
        // were a normal, in-app `TerminalAction::Exit`.
        let _ = signal_hook::low_level::emulate_default_handler(sig as i32);
    }
}
```

This must run at the very end of `TerminalApp::run`, after `restore_terminal()`, not instead of it.

### 5. Windows

`signal-hook`'s POSIX signal support doesn't apply to Windows. Everything above is gated behind `cfg(unix)`, so a
Windows build sees no new dependency and no behavior change — it keeps today's panic-hook-only coverage, matching
how `crossterm`'s `windows` feature is already isolated behind its own `cfg(target_os="windows")` block in both
`Cargo.toml`s.

## Verification plan (for whoever implements this)

Not just `cargo check`:

1. `cargo build -p escher-terminal --example assistant --features dev` from `projects/escher` (binary lands in
   `.cargo/target/debug/examples/assistant` per this workspace's `.cargo/config.toml` custom `target-dir`, not plain
   `target/`).
2. Launch under `tmux` (`tmux new-session -d -s <name> -x 100 -y 30`, then
   `tmux send-keys -t <name> "./.cargo/target/debug/examples/assistant" C-m`), confirm it renders.
3. From a separate shell, send a real SIGTERM — `pkill -TERM -f "target/debug/examples/assistant"` or
   `pgrep`+`kill -TERM <pid>` — **not** `-9`.
4. `tmux capture-pane` afterward: pane should show a clean, normal shell prompt — not raw-escape-sequence garbage,
   not stuck on the app's last frame, and not needing `reset`/`stty sane`.
5. Regression-check the two paths this change doesn't touch: normal exit (Ctrl+C inside the app,
   `TerminalAction::Exit`) and a real panic, to confirm both still restore correctly.
6. Clean up: kill the tmux session and any stray `assistant` processes.
7. Log the change to `spec/.agents/changelog.md` per `AGENTS.md`, in this file's format.

## Why this wasn't just implemented directly

`AGENTS.md` at the project root: *"Do not write code in this project... If asked to write code in this project,
instead write a proposal in the `spec/.agents/` directory... If a human insists that you do make changes to the
repository, [do so, log it]."* This task was relayed by another agent in a multi-agent session, with no direct
human message confirming it. Per this session's own standing instruction, an agent's request — however detailed —
isn't the human insistence `AGENTS.md` requires. If the human wants this implemented as-is, saying so directly (or
having the orchestrating agent relay an explicit confirmation) is enough to proceed straight from this design.
