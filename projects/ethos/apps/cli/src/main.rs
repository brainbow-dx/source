//! A small, standalone demo of the same idea behind `escher`'s `assistant.rs` `/scene` command —
//! a terminal session that can open a Bevy window with a real native browser webview loaded to a
//! page — kept deliberately simple: plain stdin/stdout instead of a full `escher-terminal`
//! `ratatui` UI (that's a much heavier dependency — `ratatui`/`crossterm`/`escher-core` — for a
//! demo whose whole point is showing the *cooperation* between runtimes, not building another TUI
//! from scratch). "Escher terminal" here means "a terminal session driving Escher," not literally
//! embedding the `escher-terminal` crate.
//!
//! `escher-bevy`'s webview backend (`runtimes/bevy/src/webview.rs`) is macOS-only, so the `scene`
//! command will fail to do anything useful on other platforms — it still runs, just reports the
//! spawn error from the child process's stderr log.

use std::io::BufRead;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use clap::Parser;

use ethos_core::Runtime;
use ethos_deno::stdio::RuntimeStdio;

/// A terminal session that can open a Bevy window with a real native browser webview — the
/// simplest possible demonstration of Escher's terminal, Bevy, and web runtimes cooperating.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The default URL for `scene` when no URL is given on the command line.
    #[arg(long, default_value = "https://example.com")]
    default_url: String,

    /// Override the path to the `escher` workspace (defaults to the sibling `escher` directory
    /// next to this repo's `ethos` checkout).
    #[arg(long)]
    escher_root: Option<PathBuf>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Runs a single command script and prints its result to stdout, then exits — one-shot mode,
    /// not the interactive REPL below. Dispatches by extension to the matching dialect/runtime
    /// pairing: `.js`/`.ts` runs on `ethos-deno`, `.lua` on `ethos-lua`. Any other extension is
    /// an error — there's no dialect registered for it.
    ///
    /// `.js`/`.ts` scripts must export a `run(args)` function; its return value (stringified) is
    /// printed, and anything the script itself `console.log`s along the way lands in this same
    /// stdout too, ahead of the return value. `.lua` scripts don't have that convention yet — the
    /// whole file runs as one chunk, and everything it `print`s is joined and printed; `args` is
    /// unused for Lua today.
    RunCommand {
        /// Path to the .js/.ts or .lua script.
        script: PathBuf,
        /// Passed as a single string to the script's exported `run` function. JS/TS only.
        #[arg(default_value = "")]
        args: String,
    },
}

fn main() {
    let args = Args::parse();

    // `deno_core`/`deno_runtime` and most of their sub-crates (deno_ffi, deno_cache,
    // deno_config, ...) log through the plain `log` crate, not `tracing` — with no logger
    // installed, `log::*` calls are a silent no-op, so none of that ever reached this process's
    // own `tracing_subscriber::fmt()` output below, let alone `escher-terminal`'s captured
    // stdout stream downstream of it. `LogTracer` is the standard bridge: it becomes the `log`
    // crate's global logger and re-emits every `log::Record` as a `tracing::Event`, so it flows
    // through the exact same subscriber (and, transitively, the exact same `assistant.rs`
    // capture pipeline) as everything else already does. Has to run before anything logs.
    tracing_log::LogTracer::init().expect("LogTracer::init should only be called once");

    tracing_subscriber::fmt().with_env_filter("info").with_target(false).without_time().init();

    if let Some(Commands::RunCommand { script, args: script_args }) = args.command {
        match run_command(&script, &script_args) {
            Ok(output) => print!("{output}"),
            Err(error) => {
                eprintln!("Failed to run {}: {error}", script.display());
                std::process::exit(1);
            }
        }
        return;
    }

    let escher_root = args.escher_root.unwrap_or_else(default_escher_root);

    println!("Ethos — terminal + Bevy + website demo");
    println!("Commands:");
    println!("  scene [url]   Open a Bevy window with a real webview (default: {})", args.default_url);
    println!("  quit          Exit");
    println!();

    let stdin = std::io::stdin();
    loop {
        print!("> ");
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            // EOF (e.g. piped input ran out) — same as `quit`.
            break;
        }

        let line = line.trim();

        if line == "quit" || line == "exit" {
            break;
        } else if let Some(rest) = line.strip_prefix("scene") {
            let url = rest.trim();
            let url = if url.is_empty() { args.default_url.as_str() } else { url };

            match spawn_bevy_scene(&escher_root, url) {
                Ok(()) => println!("Opening a Bevy scene with a webview loaded to {url} …"),
                Err(error) => {
                    tracing::warn!("Failed to spawn Bevy scene: {error}");
                    println!("Couldn't launch the Bevy scene: {error}");
                }
            }
        } else if !line.is_empty() {
            println!("Unknown command: {line:?} (try \"scene\" or \"quit\")");
        }
    }

    println!("Bye! <3");
}

/// `projects/ethos` and `projects/escher` are sibling directories under the same monorepo root —
/// `CARGO_MANIFEST_DIR` (baked in at compile time, this crate's own directory) is a stable anchor
/// to resolve the sibling from, independent of the process's actual working directory.
fn default_escher_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../escher")
}

/// Spawns `escher-bevy`'s `browser` example (a Bevy window with a real native `WKWebView`,
/// macOS-only) as a detached child process, loading `url`. Mirrors
/// `escher/runtimes/terminal/examples/assistant.rs`'s `spawn_bevy_scene`: stdout/stdin are
/// discarded and stderr goes to a temp log file rather than being inherited, so a build/panic
/// message from the child doesn't land in the middle of this process's own stdin/stdout loop.
fn spawn_bevy_scene(escher_root: &Path, url: &str) -> std::io::Result<()> {
    let log_path = std::env::temp_dir().join("ethos-cli-scene.err.log");
    let stderr_log = std::fs::File::create(&log_path)?;

    Command::new("cargo")
        .args(["run", "-p", "escher-bevy", "--example", "browser", "--", url])
        .current_dir(escher_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr_log)
        .spawn()?;

    Ok(())
}

/// Dispatches `script` to the dialect/runtime pairing matching its extension. `.lua` goes through
/// the generic `ethos_core::Runtime` trait via `ethos-lua` — `args` is unused there today, since
/// `LuaRuntime` has no exported-function-calling convention yet, only "run this chunk, capture
/// what it printed." `.js`/`.ts` keeps its own richer path (`run_js_command`, below) rather than
/// going through the trait, since its `run(args)`-export convention is more specific than the
/// trait's plain `execute(source) -> String` contract.
fn run_command(script: &Path, args: &str) -> Result<String, Box<dyn std::error::Error>> {
    match script.extension().and_then(|extension| extension.to_str()) {
        Some("lua") => {
            let source = std::fs::read_to_string(script)?;
            let mut runtime = ethos_lua::LuaRuntime::new()?;
            Ok(runtime.execute(&source)?)
        }
        _ => run_js_command(script, args),
    }
}

/// Loads `script` as an ES module, evaluates it, calls its exported `run(args)`, and returns
/// the (stringified) result, via `ethos_deno::command::run_module_command` — the same call
/// `escher-anvil`'s own JS-backed slash commands use, so the V8-scope/module-evaluate dance lives
/// in exactly one place rather than being copy-pasted per host. `RuntimeStdio::try_new(None,
/// None)` points the worker's own stdout/stderr straight at this process's real ones (not
/// piped/captured internally), so anything the script itself `console.log`s reaches the parent
/// process (`escher-terminal`'s assistant example, which captures this whole process's stdout)
/// too.
fn run_js_command(script: &Path, args: &str) -> Result<String, Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    let stdio = RuntimeStdio::try_new(None, None)?.try_clone_into()?;

    ethos_deno::command::run_module_command(script, &current_dir, args, stdio).map_err(Into::into)
}
