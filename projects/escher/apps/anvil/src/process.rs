//! Running an external command (an Ethos script, or a shell backend) and getting its output into
//! `Page::Process`'s scrollback — live, line-by-line, for `run_js_command`/`run_deno_command`; held
//! until classified for `run_shell_command` (see its own doc comment for why). Split out of
//! `main.rs` to keep that file from growing further as commands multiply — `main.rs`
//! still owns `AppState`'s `spawn_js_command`/`spawn_shell_command`/`spawn_shape_command` (they
//! close over `self`'s fields, so they can't move here), but the actual subprocess mechanics live
//! in this module.

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use color_eyre::owo_colors::OwoColorize;
use ethos_deno::stdio::RuntimeStdio;

use crate::LineBuffer;
use crate::DIM;

/// Runs a JS/TS command script by embedding `ethos-deno`'s V8 runtime directly in this process —
/// through `ethos-deno`'s own API (see `run_embedded_js` below), not `deno_core`/`deno_runtime`
/// directly — linked into `escher-anvil` instead of shelled out to as a separate binary. Keeping
/// V8 out of this process "to keep the two runtimes decoupled" isn't a real constraint — when a
/// host wants an engine embedded, embed it, and doing so here means
/// JS-backed commands work with nothing but this one binary, no `ethos`/`deno` install required at
/// all. `run_deno_command`/`run_reject_router` below still shell out to the real `deno` CLI —
/// that's for a different, still-real reason (`ethos-deno`'s embedded runtime has no TypeScript-
/// stripping or JSX support at all), not this one, so those stay as they are.
///
/// The console/`Deno.stdout` stream is piped into `process_buffer` live, the same "one line at a
/// time, not only once everything's done" streaming `run_streamed_command`'s external-process
/// callers get — but the *return value* comes back directly from calling the module's exported
/// `run(args)` through V8, not by parsing anything off that stream, so unlike the old subprocess
/// version this never has to guess where "script output" ends and "the actual result" begins.
///
/// Builds a fresh, dedicated single-threaded Tokio runtime for the call — `deno_unsync` (used
/// internally by `deno_fetch`) asserts the runtime driving it is `CurrentThread`, which this
/// crate's own multi-threaded `tokio::Runtime` (`AppState::runtime`) isn't. The caller already
/// runs this whole function on a background thread (`AppState::spawn_js_command`/
/// `spawn_shape_command`), so blocking here doesn't freeze the UI. Forwarding only works because
/// this always runs inside a `live_trace` span entered by its caller — see `TranscriptLayer`.
pub(crate) fn run_js_command(script: &Path, args: &str, command_label: &str, process_buffer: &LineBuffer) -> Result<String, String> {
    process_buffer.push_line(format!("{}", format!("── {command_label} ──").truecolor(DIM.0, DIM.1, DIM.2)));

    let (stdout_reader, stdout_writer) = std::io::pipe().map_err(|error| format!("failed to create stdout pipe: {error}"))?;
    let (stderr_reader, stderr_writer) = std::io::pipe().map_err(|error| format!("failed to create stderr pipe: {error}"))?;

    // Drained on their own threads, same reason `stream_spawned_child` does — a script that
    // writes a lot to one stream while this thread blocks reading the other can't deadlock both
    // of them on a full pipe buffer.
    let stdout_process_buffer = process_buffer.clone();
    let stdout_handle = std::thread::spawn(move || {
        BufReader::new(stdout_reader)
            .lines()
            .map_while(Result::ok)
            .for_each(|line| {
                tracing::info!("{line}");
                stdout_process_buffer.push_line(line);
            })
    });
    let stderr_process_buffer = process_buffer.clone();
    let stderr_handle = std::thread::spawn(move || {
        BufReader::new(stderr_reader)
            .lines()
            .map_while(Result::ok)
            .for_each(|line| {
                tracing::warn!("{line}");
                stderr_process_buffer.push_line(line);
            })
    });

    let result = run_embedded_js(script, args, pipe_writer_into_file(stdout_writer), pipe_writer_into_file(stderr_writer));

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    result
}

/// The actual embed — entirely through `ethos-deno`'s own API (`RuntimeStdio`, and
/// `ethos_deno::command::run_module_command` for the `MainWorker`/module-evaluate/call-`run`-
/// export dance), not `deno_core`/`deno_runtime` directly — Anvil goes through Ethos's own Deno
/// support rather than reach past it into `deno_core` internals. That dance is genuinely shared
/// with `ethos-cli`'s own `run-command`, so it lives in `ethos-deno` once, not copy-pasted per
/// host.
/// `std::io::pipe`'s own `PipeWriter` isn't a `File`, but converts losslessly into one via the
/// same owned-fd/-handle route `deno_io`'s own pipe type takes (see that crate's `pipe.rs`) —
/// `RuntimeStdio::try_new` wants a plain `File` so it isn't coupled to any one pipe
/// implementation.
#[cfg(unix)]
fn pipe_writer_into_file(writer: std::io::PipeWriter) -> File {
    File::from(std::os::fd::OwnedFd::from(writer))
}
#[cfg(windows)]
fn pipe_writer_into_file(writer: std::io::PipeWriter) -> File {
    File::from(std::os::windows::io::OwnedHandle::from(writer))
}

fn run_embedded_js(script: &Path, args: &str, stdout: File, stderr: File) -> Result<String, String> {
    let stdio = RuntimeStdio::try_new(Some(stdout), Some(stderr))
        .and_then(|stdio| stdio.try_clone_into())
        .map_err(|error| format!("failed to set up the embedded JS engine's stdio: {error}"))?;

    ethos_deno::command::run_module_command(script, &crate::anvil_root(), args, stdio)
}

/// Runs a `.tsx`/`.ts` file via the real `deno` CLI — not `ethos-cli run-command`, which goes
/// through `ethos-deno`'s embedded runtime. That runtime has no TypeScript-stripping pass at all
/// (see `escher/spec/ROADMAP.md`'s M4 for the confirmed gap) and no JSX support, so anything using
/// real TS syntax or JSX (like `commands/shape.tsx` — see `AppState::spawn_shape_command`, or
/// `commands/route.ts` — see `AppState::spawn_shell_command`) has to run through a real Deno CLI
/// invocation instead. `current_dir` is set to this crate's own root (`apps/anvil`, which has its
/// own `deno.json` and is a member of Escher's Deno workspace — see the root `deno.jsonc`) so
/// Deno's own config/import-map discovery resolves `@escher/jsx`/`@escher/core` correctly;
/// `script` is relative to that. `args`, if non-empty, is passed as the script's one `Deno.args[0]`
/// — opaque to this function, script-defined shape, same "one string, script decides what it
/// means" convention `run_js_command` already uses. `--allow-net --allow-env` are granted
/// unconditionally rather than threading a permissions parameter through every call site — every
/// script this function runs is first-party, repo-local code, not untrusted input, so this isn't a
/// real security boundary to begin with. Streams into `process_buffer` the same way
/// `run_js_command` does.
pub(crate) fn run_deno_command(script: &Path, args: &str, command_label: &str, process_buffer: &LineBuffer) -> Result<String, String> {
    let anvil_root = crate::anvil_root();

    let mut command = Command::new("deno");
    command.arg("run").arg("--allow-net").arg("--allow-env").arg(script);
    if !args.is_empty() {
        command.arg(args);
    }
    command.current_dir(anvil_root);

    run_streamed_command(command, command_label, process_buffer)
}

/// `run_reject_router`'s result — either half can be `None` on its own (a real command matched,
/// nothing left to say; or the model had a reply but it wasn't a command; or truly neither, see
/// that function's own doc comment on failure handling). Never both `Some` at once — `commands/
/// route.ts`'s own final block only ever fills in `reply` once `replace` came back empty.
pub(crate) struct RejectRouterResult {
    /// A `/command args` string to offer the user (populated into the input box, never run
    /// automatically) — see `commands/route.ts`'s `tier1LocalToolRouter`.
    pub replace: Option<String>,
    /// The model's own conversational reply — a typo correction, an answer to a greeting or
    /// question, whatever — worth showing the user even though nothing resolved to a real command.
    pub reply: Option<String>,
}

/// Runs `commands/route.ts` — a router script for text the shell fallback rejected outright (see
/// `AppState::spawn_shell_command`'s `ShellOutcome::Rejected` handling). Passes only the raw
/// rejected text — nothing about what commands exist, what a "tool" is, or what backend (a
/// local Ollama model) might do the matching. That entire domain belongs to
/// the scripting layer, including discovering what's routable in the first place (the script scans
/// `commands/` itself — see its own doc comment), so it stays portable and hot-swappable without
/// ever touching Rust, right down to Rust not knowing the request/response shape is about "tools"
/// at all. This function's only job is "run this script with this one string, read back one line of
/// JSON holding an optional replacement string and an optional reply." Doesn't go through
/// `run_deno_command`/`run_streamed_command` — this has to run silently, with nothing reaching
/// `process_buffer`: shell rejection already happens often during normal typing (see
/// `ShellOutcome::Rejected`'s own doc comment), and this is a nice-to-have suggestion layered on top
/// of it, not a real command a user asked to run. Returns an empty result on literally any failure —
/// script error, non-zero exit, unparseable output, or the script simply not matching anything — a
/// missing suggestion is always a safe, silent *UI* outcome (never disruptive, never a scrollback
/// line), but every real-failure branch below still logs via `tracing::warn!` — "silent to the UI"
/// and "invisible to the developer" turned out not to be the same requirement: a genuine bug (bad
/// path, `deno` not on `PATH`, a script error) needs to be diagnosable via `Page::Trace`, not
/// indistinguishable from the script simply declining to match anything. `--allow-read` (alongside
/// `--allow-net --allow-env`) is what lets the script scan `commands/` on its own instead of Rust
/// building that list for it.
pub(crate) fn run_reject_router(script: &Path, prompt: &str) -> RejectRouterResult {
    let empty = RejectRouterResult { replace: None, reply: None };
    let anvil_root = crate::anvil_root();

    let output = match Command::new("deno")
        .args(["run", "--allow-net", "--allow-env", "--allow-read"])
        .arg(script)
        .arg(prompt)
        .current_dir(anvil_root)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!("Failed to spawn `deno run {}`: {error}", script.display());
            return empty;
        }
    };

    if !output.status.success() {
        tracing::warn!("`deno run {}` exited with {}: {}", script.display(), output.status, String::from_utf8_lossy(&output.stderr).trim());
        return empty;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(line) = stdout.lines().next_back() else {
        tracing::warn!("`deno run {}` produced no output", script.display());
        return empty;
    };
    let parsed: serde_json::Value = match serde_json::from_str(line) {
        Ok(parsed) => parsed,
        Err(error) => {
            tracing::warn!("`deno run {}` produced unparseable output {line:?}: {error}", script.display());
            return empty;
        }
    };

    // A real, expected outcome — the script found no match — is `null`/absent, not an error.
    RejectRouterResult {
        replace: parsed.get("replace").and_then(serde_json::Value::as_str).map(str::to_string),
        reply: parsed.get("reply").and_then(serde_json::Value::as_str).map(str::to_string),
    }
}

/// Outcome of `run_shell_command`: either the shell actually attempted `prompt` (whether it then
/// succeeded or failed on its own terms), or it rejected the text outright before doing anything.
/// Every common shell (`sh`/`bash`/`zsh`/`dash`) exits 127 specifically for "command not found" —
/// the near-certain signature of input that was never a command at all (a mistyped slash command,
/// stray text). Kept as its own variant, not folded into `Result<String, String>`'s `Err`, so the
/// caller (`AppState::spawn_shell_command`) can treat "the shell said nah" as a non-event — no
/// scrollback line, no chat message, no lingering status — rather than a real failed command worth
/// a record of.
pub(crate) enum ShellOutcome {
    Ran(Result<String, String>),
    Rejected,
}

/// Runs `prompt` as a single command against `shell`, same as a real terminal's "run whatever I
/// typed" — used by the input handler's shell-passthrough fallback (unrecognized input that
/// isn't a known slash command). `shell` is resolved by the caller (`AppState::spawn_shell_command`,
/// via `resolve_shell_backend` below), not here, so this stays a pure "run this shell with this
/// input" primitive.
///
/// Streams live into `process_buffer` via `stream_spawned_child`, same as `run_streamed_command` —
/// but only once `REJECTION_POLL_WINDOW` has passed with the child still alive, or it's already
/// exited with something other than 127. Holding *all* output until the
/// whole command finished (the original design here) made even a fast, valid command look frozen
/// for however long it took to run, when the actual thing that needs deciding first — "did the
/// shell even recognize this at all" — resolves almost instantly. "Command not found" is a
/// synchronous `PATH` lookup failure inside the shell itself: no real work ever starts, so a
/// genuinely rejected prompt (a typo, stray text) still exits 127 within a handful of
/// milliseconds, comfortably inside the poll window — so this still leaves rejected input with no
/// trace in `Page::Process` at all, exactly like before, without making every *valid* command wait
/// for its own full run just to prove it wasn't a typo.
pub(crate) fn run_shell_command(shell: &str, prompt: &str, process_buffer: &LineBuffer) -> ShellOutcome {
    let mut command = Command::new(shell);
    command.arg("-c").arg(prompt);

    let mut child = match command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(error) => return ShellOutcome::Ran(Err(format!("failed to spawn command: {error}"))),
    };

    const REJECTION_POLL_WINDOW: Duration = Duration::from_millis(80);
    const REJECTION_POLL_INTERVAL: Duration = Duration::from_millis(5);
    let deadline = Instant::now() + REJECTION_POLL_WINDOW;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.code() == Some(127) => return ShellOutcome::Rejected,
            Ok(Some(_)) => break, // Exited already, but not a rejection — stream/collect its (short) output normally below.
            Ok(None) if Instant::now() >= deadline => break, // Still running past the window — long enough to trust, start streaming.
            Ok(None) => std::thread::sleep(REJECTION_POLL_INTERVAL),
            Err(error) => return ShellOutcome::Ran(Err(format!("failed waiting for command: {error}"))),
        }
    }

    ShellOutcome::Ran(stream_spawned_child(child, prompt, process_buffer))
}

/// The actual spawn/stream/wait machinery shared by `run_js_command` (an `ethos-cli run-command`
/// invocation) and `run_shell_command` (a configured shell backend) — everything past "here's a
/// `Command`, ready to run" is identical between the two: a header line into `process_buffer`,
/// stdout/stderr each streamed line-by-line into it from separate threads (so a child that writes
/// a lot to one stream while this thread blocks reading the other can't deadlock both of them on
/// a full pipe buffer), `tracing::info!`/`warn!` forwarding, and the same "stderr on failure,
/// stdout on success" `Result` shape.
fn run_streamed_command(mut command: Command, command_label: &str, process_buffer: &LineBuffer) -> Result<String, String> {
    let child = command
        // Anvil's own raw-mode event loop already owns and consumes all keyboard input for its
        // own widgets — a child inheriting it would never actually receive anything a user types,
        // just block forever waiting for input that can't arrive. `Stdio::null()` turns that into
        // an immediate EOF, same as running a script with `< /dev/null`, so a child that reads
        // stdin (`cat`, a REPL, `read`) exits promptly instead of hanging this task forever.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn command: {error}"))?;

    stream_spawned_child(child, command_label, process_buffer)
}

/// The live-streaming half of `run_streamed_command`, split out so `run_shell_command` can reuse it
/// against a child it already spawned (and briefly polled for an early rejection) itself, rather
/// than duplicating this exact stdout/stderr-draining dance a second time.
fn stream_spawned_child(mut child: std::process::Child, command_label: &str, process_buffer: &LineBuffer) -> Result<String, String> {
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    // A header line per run (dim, not part of the child's own output) so `Page::Process`'s
    // continuous scrollback across every command this session stays legible about where one
    // run ends and the next begins — the raw stdio itself carries no such marker on its own.
    process_buffer.push_line(format!("{}", format!("── {command_label} ──").truecolor(DIM.0, DIM.1, DIM.2)));

    // Stderr drains on its own thread so a child that writes a lot to one stream while this
    // thread blocks reading the other can't deadlock both of them on a full pipe buffer. Each
    // line reaches two places independently: `tracing::warn!` (the existing `live_trace`-scoped
    // forwarding into the chat transcript) and `process_buffer` (the exact raw line, no `tracing`
    // formatting/timestamp/level prefix at all, pushed directly from both threads as lines
    // actually arrive, so `Page::Process`'s stdout/stderr interleaving tracks real arrival order
    // the same way a real terminal's combined stream would, not just concatenated after the fact).
    let stderr_process_buffer = process_buffer.clone();
    let stderr_handle = std::thread::spawn(move || {
        BufReader::new(stderr)
            .lines()
            .map_while(Result::ok)
            .inspect(|line| {
                tracing::warn!("{line}");
                stderr_process_buffer.push_line(line.clone());
            })
            .collect::<Vec<_>>()
    });

    let stdout_lines: Vec<String> = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .inspect(|line| {
            tracing::info!("{line}");
            process_buffer.push_line(line.clone());
        })
        .collect();

    let stderr_lines = stderr_handle.join().unwrap_or_default();

    let status = child.wait().map_err(|error| format!("failed waiting for command: {error}"))?;

    if !status.success() {
        return Err(stderr_lines.join("\n").trim().to_string());
    }

    Ok(stdout_lines.join("\n").trim_end().to_string())
}

/// The shell backend the shell-passthrough feature (unrecognized input → real shell) runs
/// against — `$ANVIL_SHELL` if set, else `$SHELL` (a real login shell already configured on this
/// machine), else `/bin/sh` as a last-resort default so the feature always has *something* to run
/// against. "Configured by the user," per the feature request, means exactly this env var today —
/// not a new config-file format, which would be more machinery than a one-var setting needs.
pub(crate) fn resolve_shell_backend() -> String {
    std::env::var("ANVIL_SHELL")
        .or_else(|_| std::env::var("SHELL"))
        .unwrap_or_else(|_| "/bin/sh".to_string())
}

