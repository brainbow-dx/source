extern crate alloc;

mod config;
mod persistence;
mod process;
mod shape;

use std::process::ExitCode;
use std::io;
use std::io::Stdout;
use std::fmt::Write as _;
use std::io::Write as _;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::Instant;

use alloc::sync::Arc;

use color_eyre::Result;
use color_eyre::owo_colors::OwoColorize;

use clap::Parser;

use parking_lot::Mutex;
use parking_lot::RwLock;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use unicode_width::UnicodeWidthStr;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::crossterm::event::Event as CrosstermEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::crossterm::event::MouseEventKind;

use escher_core::element::*;
use escher_core::style::*;
use escher_core::content::LineCounter;

use escher_terminal::app::TerminalAction;
use escher_terminal::surface::TerminalSurface;
// Re-exported so `process.rs`/`shape.rs`'s existing `use crate::LineBuffer;` keeps working
// unchanged — the type itself lives in `escher-terminal` (see that crate's `tracing_bridge`
// module), it was never actually Anvil-specific.
pub(crate) use escher_terminal::tracing_bridge::LineBuffer;
use escher_terminal::text_wrap::wrap_hanging;
use escher_terminal::text_wrap::wrap_words;

// `/browser` opens its window *in this same process* — see `AssistantTerminalPlugin`'s own doc
// comment for why that means this whole app runs as a Bevy app now, not a standalone
// `TerminalApp::run` loop. Explicit imports, not `bevy::prelude::*` — several names in there
// (`Color`, `Overflow`, `ScrollPosition`, `FlexDirection`, `BackgroundColor`) collide with
// `escher_core::style`'s own types of the same name, already used unqualified everywhere else in
// this file's `Scaffold`-building code.
use bevy::app::App;
use bevy::app::AppExit;
use bevy::app::Last;
use bevy::app::Plugin;
use bevy::app::PreUpdate;
use bevy::app::Startup;
use bevy::app::Update;
use bevy::color::Color as BevyColor;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::MessageReader;
use bevy::ecs::message::MessageWriter;
use bevy::ecs::query::With;
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::Commands;
use bevy::ecs::system::NonSend;
use bevy::ecs::system::NonSendMut;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::ecs::system::ResMut;
use bevy::camera::Camera2d;
use bevy::camera::ClearColorConfig;
use bevy::sprite::Text2d;
use bevy::text::TextColor;
use bevy::text::TextFont;
use bevy::window::RawHandleWrapper;
use bevy::window::Window;

use escher_bevy::os::OsPlugin;
use escher_bevy::terminal::spawn_input_watcher;
use escher_bevy::webview::SceneCommand;
use escher_bevy::EscherBevyConfig;
use escher_bevy::EscherBevyPlugin;

use escher_appkit::bevy::{
    TabInfo, TabStripEvent, TabStripState, ThemeState, ToolbarEvent, ToolbarPlugin, ToolbarState, ToolbarSystems, ToolbarTheme, WantsTabStrip, WantsToolbar,
    ICON_ONLY_WIDTH, MAX_WIDTH, MIN_WIDTH, RESIZE_HANDLE_WIDTH, TOOLBAR_HEIGHT,
};

/// How much of the browser window's left edge the tab strip (plus its own resize handle) claims
/// — what a webview's `left_inset` has to match. Collapsed (`icon_only`), the resize handle is
/// hidden (nothing to drag: the icon rail's width is a fixed constant, not something a user
/// picks — see `TabStripState::icon_only`'s own doc comment), so its width isn't reserved either;
/// reserving it anyway with nothing drawn there left a bare, undrawn strip of the window's own
/// backing (reading as a stray black bar) between the icon rail and the webview.
fn tab_strip_content_inset(tab_strip: &TabStripState) -> f64 {
    let width = tab_strip.effective_width();
    if tab_strip.icon_only() {
        width
    } else {
        width + RESIZE_HANDLE_WIDTH
    }
}

// Anvil: an inventor's notebook built entirely out of Escher scaffolds — an AI-assistant-style
// terminal UI (a scrollable transcript of user/assistant/tool turns; PageUp/PageDown, not the
// terminal emulator's own scrollback, which generally doesn't work for a raw-mode/redrawing TUI —
// the app owns its own scroll position instead) above a bordered input prompt, with a real native
// webview + chrome bar living alongside it in the same process (`/browser <url>`). Doubles as a
// running demo of what Escher's terminal/Bevy/webview/OS-integration surfaces can do together,
// and the intended eventual home for a full editor/workspace-management tool for Ethos/Escher/
// Atlas apps. Follows `escher-terminal`'s `ratatui.rs` example's structure (TerminalApp/
// TerminalSurface, Header/Body/Footer slots, a keyboard handler feeding an Arc<RwLock<..>> input
// buffer), plus real ANSI color (rendered via `ansi_to_tui`, same as any other Escher content)
// and a Tab-toggled expand/collapse on tool calls.

//---
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// A handful of dependencies are silenced below the app's own default — at plain `trace`,
    /// `wgpu_core`/`wgpu_hal` log every single GPU call (`Device::create_bind_group`,
    /// `Queue::submit`, ...), `naga` dumps its full numeric-overload-resolution rule table on
    /// every shader type-check, `bevy_shader` logs every shader-def permutation it processes,
    /// `winit` traces every single AppKit window-delegate callback, `hyper`/`libsql_sys`/
    /// `libsql_replication`/plain `libsql` (the connection/statement layer itself — `preparing`/
    /// `query for prepared statement`) trace every byte of the `sqld` replication connection's
    /// HTTP/WAL traffic, `tower_http` traces every gRPC request/response on that same
    /// connection, and `libsql-sqlite3-parser`'s generated lexer/LALR parser traces every token
    /// scanned (target `"scanner"`) and every shift/reduce/pop step of every SQL statement the
    /// embedded replica prepares. That LALR parser trace is by far the dominant source — two
    /// wrong guesses at its real target got made and caught in this same session before landing
    /// on the right one: first the crate's own module path (`libsql_sqlite3_parser`, matched
    /// nothing), then the literal string `"Parse"` (`lempar.rs`'s `static TARGET: &str =
    /// "Parse"` — *also* matched nothing, confirmed live: an 8-second session was still 97%
    /// this one target even with that "fix" in place). The actual answer needed reading the
    /// crate's *generated* output, not its checked-in template source: `lempar.rs` is only the
    /// template `libsql-sqlite3-parser`'s own `build.rs` fills in with this specific SQL
    /// grammar's tables, and that build step overrides `TARGET` per grammar — the real,
    /// generated `target/debug/build/libsql-sqlite3-parser-*/out/parse.rs` sets `static TARGET:
    /// &str = "sqlite3Parser"`. Live-verified this one, unlike the previous two: an 8-second
    /// session's `anvil.log` line count actually dropped once `sqlite3Parser=warn` replaced
    /// `Parse=warn`. None of this is app-level tracing, and all of it fires every frame,
    /// request, or query. `wgpu`/`naga` use the same `"wgpu=error,naga=warn"` suppression
    /// Bevy's own `LogPlugin` recommends by default; the rest were found by watching `anvil.log`
    /// balloon (millions of lines within an hour of normal use) and actually reading each
    /// culprit's own *generated* tracing call sites for its real target, not assuming one from
    /// its crate name or its template source.
    #[arg(
        short,
        long,
        default_value = "trace,wgpu=error,naga=warn,bevy_shader=warn,winit=warn,hyper=warn,tower_http=warn,libsql=warn,libsql_sys=warn,libsql_replication=warn,scanner=warn,sqlite3Parser=warn"
    )]
    log_level: String,

    /// Skip the TUI at startup and just print the raw, unformatted trace stream straight to the
    /// terminal instead — the same thing F5 switches to at runtime (see `RawStreamGate`), useful
    /// when a `Scaffold`/`TerminalSurface` bug means the TUI itself can't be trusted to render.
    #[arg(long, default_value_t = false)]
    no_tui: bool,

    /// Print this run's captured trace output (from this session's own `anvil.log` — see
    /// `anvil_log_dir`) to stdout after exiting. Otherwise the only way to see what happened
    /// during a run is `tail -f` that file in a second terminal while it's still open — the
    /// alternate screen this app draws to hides its own stdout, and the log file's own content
    /// stops being reachable once the process exits.
    #[arg(long, default_value_t = false)]
    dump_trace: bool,

    /// Wipes all persisted messages/tasks from `sqld` and exits — doesn't launch the TUI.
    #[arg(long, default_value_t = false)]
    reset_data: bool,

    /// Sync this session's `libsql` replica against a different `sqld` primary than the default
    /// local one — the direct-URL half of joining a co-working session someone else is hosting
    /// (their machine's own local `sqld`, its source of truth, becomes yours too). A short "room
    /// code" resolving through Atlas's own peer discovery instead of a raw URL is the intended
    /// friendlier front end for this same flag, not yet built — this is deliberately the
    /// troubleshooting-grade escape hatch underneath it, and works standalone today for anyone who
    /// already knows (or can reach, e.g. over Tailscale) the host's address directly, such as
    /// `http://100.x.y.z:8081`.
    #[arg(long)]
    connect: Option<String>,

    /// Announces this instance's identity. Each running `anvil` (one per person, in a co-working
    /// session) needs an easy way to be told apart, e.g. so its own
    /// overlay window position (`overlay_state`, keyed by this) doesn't overwrite anyone else's.
    /// Defaults to `anvil-<pid>` when not given, matching the same pid-keyed convention this app
    /// already uses for its session directory and log file (see `anvil_session_dir`/
    /// `anvil_log_dir`) — good enough
    /// to tell instances apart locally; pass this explicitly for a name that actually means
    /// something once instances are spread across machines.
    #[arg(long)]
    identity: Option<String>,

    /// Keeps the `/browser`/`/scene` windows floating above every other window, not just this
    /// process's own, for as long as this instance runs. A real, per-instance choice rather than
    /// tied to build mode — a release build is exactly when someone testing the app still wants
    /// this, so it isn't hardcoded to debug builds only. `false` by default unless this flag or
    /// `.anvil.toml`'s `[window] always_on_top` says otherwise (this flag wins if both are
    /// given). Either way, a newly-opened window still gets focused once at creation regardless of
    /// this setting — see `FocusPending`.
    #[arg(long)]
    always_on_top: bool,
}

/// See `config::run_init`'s own doc comment for what this actually does.
#[derive(clap::Subcommand, Debug)]
enum Command {
    Init {
        /// Skips probing/`docker compose` for `sqld` entirely — trusts this address outright.
        /// The escape hatch for pointing at one that isn't reachable from this machine yet (a
        /// teammate's host, say), or just isn't running locally at all.
        #[arg(long)]
        sqld_url: Option<String>,
        /// Same idea as `--sqld-url`, for Ollama.
        #[arg(long)]
        ollama_url: Option<String>,
    },
}


//---
/// Hosts this whole app's terminal UI *inside* a Bevy app, instead of `TerminalApp::run` owning
/// its own event loop — the only way `/browser`/`/scene` can open a real in-process window (see
/// `main`):
/// Bevy's winit event loop needs the main thread, and so does a normal `TerminalApp::run` loop,
/// so the two can't coexist as two separate loops in one process. This flips it: Bevy owns the
/// main thread, this plugin's `draw_ui` runs as a guest inside Bevy's own `PreUpdate` schedule.
/// Mirrors `escher_bevy::terminal::TerminalPlugin`/`TerminalProvider` closely (same setup/drop/
/// input-watcher shape — see `spawn_input_watcher`'s own doc comment there for why it's needed at
/// all) — that one hosts its own small hardcoded UI to prove the mechanism; this one calls the
/// real `draw_assistant` instead, so none of this app's actual logic had to change to get here.
struct AssistantTerminalPlugin;

impl Plugin for AssistantTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, assistant_terminal_startup);
        app.add_systems(PreUpdate, assistant_terminal_draw);
        app.add_systems(Last, assistant_terminal_exit);
    }
}

/// Just the surface + signal-handling state — `AppState` (all of this app's actual data) is its
/// own separate `Resource`, inserted directly by `main` (it's built there already, the same way
/// it always was, since `tracing` has to be live — and so does everything `AppState::new`'s
/// background persistence-connect logs through it — before the Bevy `App` exists at all).
#[derive(Resource)]
struct TerminalHandle {
    surface: TerminalSurface<CrosstermBackend<Stdout>>,
    /// Whether the alternate screen is currently given up in favor of `RawStreamGate`'s plain
    /// trace stream — mirrors `AppState::raw_stream` but tracked here too (rather than read
    /// fresh each tick) so `assistant_terminal_draw` can tell a *transition* apart from "still in
    /// the same mode as last tick" and only enter/leave the alternate screen on the tick that
    /// actually changes, not every tick.
    in_raw_stream: bool,
    /// Set by `escher_bevy::terminal::spawn_signal_watcher` on `SIGTERM`/`SIGHUP`/`SIGINT`,
    /// checked once per `draw` tick. Without it, an external `kill`/`pkill` (no `-9`) would leave
    /// the terminal stuck in raw mode with mouse capture still enabled.
    #[cfg(unix)]
    signal_flag: Arc<std::sync::atomic::AtomicUsize>,
}

fn assistant_terminal_startup(
    mut commands: Commands,
    event_loop_proxy: Res<bevy::winit::EventLoopProxyWrapper>,
    state: Res<AppState>,
) {
    // `with_exit_on_escape(false)`: this app gives Escape its own real meaning (back out of
    // Trace/Process/a selected task, see `draw_assistant`'s `KeyCode::Esc` handler), with its own
    // separate real exit path (Ctrl+C/a signal). Without this, `TerminalSurface`'s own default
    // "Escape exits" behavior intercepts every Escape press before it reaches that handler.
    let mut surface = TerminalSurface::<CrosstermBackend<Stdout>>::try_default()
        .expect("failed to construct the terminal surface")
        .with_exit_on_escape(false);

    crossterm::terminal::enable_raw_mode().expect("failed to enable raw mode");

    // Raw mode, mouse capture, focus/paste reporting: all wanted either way (`RawStreamGate`'s
    // own poll loop still needs raw mode to catch a bare F1 press with no Enter). Only the
    // alternate screen itself is conditional — `--no-tui` means starting already in the plain
    // trace stream, so there's no `Scaffold` frame to protect stdout's normal scrollback from
    // yet, and entering it now would just have to be immediately left again.
    let start_in_raw_stream = state.raw_stream.load(Ordering::Relaxed);
    let alternate_screen_result = if start_in_raw_stream {
        crossterm::execute!(
            surface.backend_mut(),
            crossterm::event::EnableFocusChange,
            crossterm::event::EnableMouseCapture,
            crossterm::event::EnableBracketedPaste,
            crossterm::cursor::EnableBlinking,
        )
    } else {
        crossterm::execute!(
            surface.backend_mut(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableFocusChange,
            crossterm::event::EnableMouseCapture,
            crossterm::event::EnableBracketedPaste,
            crossterm::cursor::EnableBlinking,
        )
    };
    alternate_screen_result.expect("failed to prepare the terminal");

    if start_in_raw_stream {
        print_raw_stream_banner();
    }

    // See `escher_bevy::terminal::spawn_signal_watcher`'s own doc comment for why this needs a
    // real background thread rather than a bare `signal_hook::flag` handler (measured: ~13s vs.
    // ~125ms to actually exit after a real `SIGTERM`, under `WinitSettings::desktop_app()`).
    #[cfg(unix)]
    let signal_flag = escher_bevy::terminal::spawn_signal_watcher(event_loop_proxy.clone());

    spawn_input_watcher(event_loop_proxy.clone());

    commands.insert_resource(TerminalHandle {
        surface,
        in_raw_stream: start_in_raw_stream,
        #[cfg(unix)]
        signal_flag,
    });
}

/// One tick of the plain, non-TUI raw trace stream (`--no-tui` at startup, or F1 from inside the
/// TUI) — deliberately does nothing `Scaffold`/`TerminalSurface`-shaped, so a bug in that
/// rendering/dispatch code can't take this mode down too. `RawStreamGate` already prints new
/// tracing output live as it happens, with no polling needed here — this only has to notice a
/// bare F5 press (raw mode stays enabled the whole time this app runs, so it arrives without
/// Enter) to hand control back to the TUI. F5, not F1 — F1 is reserved for help/settings
/// (matching the near-universal convention), and this is a secondary,
/// leave-the-whole-TUI diagnostics tool, not the first thing to reach for (that's `Page::Trace`,
/// F2 — same firehose, nested inside the normal UI, smooth/fast/uninterrupted only matters here
/// because leaving the TUI entirely is the point when even `Page::Trace`'s own rendering is what's
/// under suspicion).
fn run_raw_stream_tick(terminal: &mut TerminalHandle, state: &AppState) {
    if !terminal.in_raw_stream {
        terminal.in_raw_stream = true;
        let _ = crossterm::execute!(terminal.surface.backend_mut(), crossterm::terminal::LeaveAlternateScreen);
        print_raw_stream_banner();
    }

    if let Ok(true) = crossterm::event::poll(Duration::from_millis(33))
        && let Ok(CrosstermEvent::Key(key)) = crossterm::event::read()
        && key.kind != KeyEventKind::Release
        && key.code == KeyCode::F(5)
    {
        state.raw_stream.store(false, Ordering::Relaxed);
    }
}

fn print_raw_stream_banner() {
    let _ = io::stdout().write_all(b"\r\n-- raw trace stream: press F5 to switch to the TUI --\r\n\r\n");
    let _ = io::stdout().flush();
}

/// Draws one frame of `draw_assistant`'s own `Scaffold` UI and dispatches whatever terminal input
/// arrived, exactly as `TerminalApp::run`'s loop body used to. `TerminalAction::Exit` (Escape, in
/// this app) and a caught signal (see `TerminalHandle::signal_flag`) both end up writing
/// `AppExit` instead of `break`ing a loop — `assistant_terminal_exit` (in `Last`) does the actual
/// teardown once Bevy's own schedule has finished processing that `AppExit`.
fn assistant_terminal_draw(
    mut terminal: ResMut<TerminalHandle>,
    state: Res<AppState>,
    mut exit_evt: MessageWriter<AppExit>,
    mut browser_evt: MessageWriter<SceneCommand>,
) {
    #[cfg(unix)]
    if terminal.signal_flag.load(std::sync::atomic::Ordering::Relaxed) != 0 {
        exit_evt.write(AppExit::Success);
        return;
    }

    // Set by `spawn_js_command` when `/quit` (see `commands/quit.js`) runs — see
    // `AppState::quit_requested`'s own doc comment for why a script needs a flag to ask for this
    // rather than just exiting itself.
    if state.quit_requested.load(Ordering::Relaxed) {
        exit_evt.write(AppExit::Success);
        return;
    }

    if state.raw_stream.load(Ordering::Relaxed) {
        run_raw_stream_tick(&mut terminal, &state);
        return;
    }

    if terminal.in_raw_stream {
        // Just switched back from the raw stream (the tick above flipped `raw_stream` to
        // false) — re-enter the alternate screen before the first `Scaffold` draw below, so it
        // renders into a clean buffer instead of on top of the plain scrollback text that was
        // just printing there. `clear()` on top of that is load-bearing, not defensive: ratatui
        // diffs each draw against its own internal idea of what's already on screen, and that
        // idea is now stale — this surface left and re-entered the alternate screen behind
        // ratatui's back (via a raw `crossterm::execute!`, not `Terminal::clear`), so without
        // this the next draw only sends the cells that changed since the *pre-raw-stream* frame,
        // leaving whatever the raw stream printed showing through underneath the new one.
        let _ = crossterm::execute!(terminal.surface.backend_mut(), crossterm::terminal::EnterAlternateScreen);
        let _ = terminal.surface.clear();
        terminal.in_raw_stream = false;
    }

    // Drain already-pending terminal events this tick, not just one. `TerminalSurface::draw` (via
    // `draw_with_poll_timeout(.., Duration::ZERO)`, see that method's own doc comment) renders the
    // *current* state, then does one non-blocking poll+dispatch of a single (possibly Drag-
    // coalesced) event if one's already waiting — mouse-based text selection needs the just-
    // rendered frame buffer to resolve a click to a character, so render has to come before
    // dispatch (see `surface.rs`'s own comment on this).
    //
    // The pending-check has to run *before* each `draw_assistant` call, not after — a previous
    // version checked after, which looked equivalent but wasn't: since render-then-dispatch means
    // a call's dispatch is invisible until the *next* call's render, checking after a call decides
    // whether to loop again based on whether that call's own dispatch left anything else queued —
    // if it didn't, the loop broke immediately, and the dispatch that call *did* just perform was
    // never rendered at all until some unrelated tick (a cursor blink, say) happened to redraw
    // later. That's a real, reported bug (characters appearing to lag or scramble while typing),
    // not a hypothetical — every burst's last keystroke was invisible until something else nudged
    // a redraw. Checking before guarantees one extra trailing call once the queue is empty: that
    // call's render reflects everything dispatched so far (including the previous call's own
    // dispatch) and its own internal poll correctly finds nothing left, so it dispatches nothing —
    // the screen is always current by the time this tick ends, not "eventually, a tick behind."
    //
    // `MAX_DRAWS_PER_TICK` used to be 64 — enough to fully drain almost any burst, typing or drag,
    // in one tick. That was the bug: a sustained fast drag keeps the queue nonempty continuously,
    // so this loop would happily render up to 64 real, full frames back-to-back in a few
    // milliseconds of wall-clock time — measured live well over 100fps, sometimes 250+, all of it
    // wasted (a terminal has no reason to repaint faster than ~60fps; nothing past that is visible
    // to a human). Dropped to 3 — enough to drain a normal multi-keystroke typing burst in one
    // tick plus the one guaranteed trailing call, not enough to matter for repaint cost. A drag
    // burst too long to fully drain in 3 iterations now spills into *later* ticks instead of
    // spiking this one — safe only because `spawn_input_watcher` (`escher-bevy`) now re-wakes Bevy
    // roughly every 16ms for as long as the queue stays nonempty, rather than once per burst; see
    // its own doc comment for that half of this fix. Without that change, lowering this number
    // would have just traded a render spike for a stall.
    const MAX_DRAWS_PER_TICK: u32 = 3;

    for _ in 0..MAX_DRAWS_PER_TICK {
        let more_pending = crossterm::event::poll(Duration::ZERO).unwrap_or(false);

        match draw_assistant(&mut terminal.surface, &state) {
            Ok(TerminalAction::Exit(_)) => {
                exit_evt.write(AppExit::Success);
                break;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!("Terminal draw failed: {error}");
                break;
            }
        }

        if !more_pending {
            break;
        }
    }

    for url in state.pending_browser_urls.lock().drain(..) {
        browser_evt.write(SceneCommand { url });
    }
}

fn assistant_terminal_exit(terminal: Option<ResMut<TerminalHandle>>, state: Res<AppState>, mut exit_evt: MessageReader<AppExit>) {
    let Some(mut terminal) = terminal else { return };

    for _ in exit_evt.read() {
        restore_assistant_terminal(&mut terminal.surface);

        // Dropping `AppState::runtime` (further down, once `.run()` itself returns) blocks until
        // every task still running on it settles — the periodic sqld resync loop and the
        // persistence writer both included. That can take a visible moment, and with nothing on
        // screen it reads as a hang rather than real cleanup. Reusing the same `raw_stream` flag
        // `RawStreamGate` already gates on (see `AppState::raw_stream`'s own doc comment) turns
        // every `tracing::*` call from here on into a live, plain-stdout stream instead of
        // silence — the same mechanism the `F1` raw-shell mode already uses, not a new one.
        state.raw_stream.store(true, std::sync::atomic::Ordering::Relaxed);
        // Not `println!` — it panics on a write failure (unlike a plain `write!`/`writeln!`
        // call, whose `Result` this just discards), and this is a real, reproducible way to hit
        // exactly that: the terminal this process was attached to can already be gone by the
        // time shutdown runs (confirmed live — this is Anvil's own long-standing, previously
        // undiagnosed recurring exit-time crash, root-caused via `panic.log`: a broken stdout/
        // stderr pipe made a `print!`/`eprintln!` call panic during `AppExit` handling, and a
        // panic during shutdown cleanup is exactly the kind of thing that reads as "escher-anvil
        // quit unexpectedly" with no visible cause).
        let _ = writeln!(std::io::stdout(), "Shutting down — flushing any pending sqld writes...");
        tracing::info!("Anvil shutting down");

        #[cfg(unix)]
        escher_bevy::terminal::reraise_signal(&terminal.signal_flag);
    }
}

/// Disables raw mode and leaves the alternate screen — see `app.rs`'s own `restore_terminal` for
/// the full reasoning (scroll-region/SGR reset before leaving, safe to call more than once). Not
/// reused directly since it isn't public and operates on `escher_terminal::app`'s own surface
/// type in a slightly different shape than convenient to call from here.
fn restore_assistant_terminal(surface: &mut TerminalSurface<CrosstermBackend<Stdout>>) {
    let _ = crossterm::execute!(
        surface.backend_mut(),
        crossterm::event::DisableFocusChange,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::cursor::DisableBlinking,
    );

    // `eprintln!`, not `writeln!`, would panic on a write failure here — see
    // `assistant_terminal_exit`'s own doc comment for why that's not hypothetical: the terminal
    // this process was attached to can already be gone by the time shutdown runs, and this exact
    // pair of calls (root-caused via `panic.log`) is what turned that into Anvil's own
    // long-standing, previously undiagnosed recurring exit-time crash.
    if let Err(error) = crossterm::terminal::disable_raw_mode() {
        let _ = writeln!(std::io::stderr(), "Failed to disable raw mode: {error}");
    }

    let _ = crossterm::execute!(surface.backend_mut(), crossterm::style::ResetColor);
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[r\x1b[0m");
    let _ = std::io::stdout().flush();

    if let Err(error) = crossterm::execute!(surface.backend_mut(), crossterm::terminal::LeaveAlternateScreen) {
        let _ = writeln!(std::io::stderr(), "Failed to leave alternate screen: {error}");
    }
}

/// Fixed width, in points, the vertical tab strip reserves on the left of the single browser
/// window — the tab-strip counterpart to `escher_appkit::CHROME_BAR_HEIGHT`.
/// One open page — anvil's own tab bookkeeping, richer than `escher_appkit::bevy::TabInfo` (which
/// only needs what the tab strip itself renders: id/title/host). `url` is what the toolbar's
/// address field and this tab's own `WebView` need instead.
struct Tab {
    id: u64,
    url: String,
    title: String,
    host: String,
    /// This tab's own loading state — set every tick by `sync_tab_loading_state` from this tab's
    /// `WebView::is_loading()`, read by `sync_toolbar_state` (only for the active tab, today) and
    /// available for the tab strip to show per-tab loading indicators later. Lives on `Tab` itself
    /// rather than being computed fresh wherever it's needed, so a tab's own nav-relevant state
    /// stays with the tab instead of toolbar code reaching into `TabWebViews` on its own. Starts
    /// `true` from `open_tab` (see its own doc comment) rather than only becoming `true` once a
    /// `WebView` exists to ask — a brand new tab is unambiguously "about to load something" from
    /// the instant it's created, not just from whenever `WKWebView`'s own delegate happens to fire.
    loading: bool,
    /// Whether `attach_pending_tab_webviews` has already let one full frame render with this tab
    /// visible (in the tab strip, showing `loading`) before paying `WebView::attach`'s synchronous
    /// native-view-creation cost. Starts `false`; see `attach_pending_tab_webviews`'s doc comment
    /// for why a brand new tab has to wait one tick rather than attach immediately.
    attach_deferred: bool,
}

/// Every open tab for the single browser window `/browser` opens into now — supersedes the
/// earlier one-OS-window-per-scene design (see `escher/spec/.agents/changelog.md`'s matching
/// entry for why). Each tab gets its own `WebView` (see `TabWebViews`) so switching tabs shows/hides the
/// right native view instead of reloading a shared one — real per-tab page state (scroll position,
/// form input, JS state) survives a switch.
#[derive(Resource, Default)]
struct BrowserState {
    tabs: Vec<Tab>,
    active: Option<u64>,
    next_id: u64,
    window: Option<Entity>,
}

impl BrowserState {
    fn active_tab(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.tabs.iter().find(|tab| tab.id == id))
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let id = self.active?;
        self.tabs.iter_mut().find(|tab| tab.id == id)
    }
}

/// A real, working demo submenu — proves `escher_os::menu::MenuItem::Item`'s action-click wiring
/// (previously inert), and exercises clipboard/dialog/sound together from one place a user can
/// actually click, not just call from code. Appended after the standard App/Edit menus via
/// `OsPlugin::with_extra_menu_items`.
fn demo_menu() -> escher_os::menu::MenuItem {
    use escher_os::menu::MenuItem;
    use std::sync::Arc;

    MenuItem::Submenu {
        label: "Demo".to_string(),
        items: vec![
            MenuItem::Item {
                label: "Say Hello".to_string(),
                key_equivalent: String::new(),
                action: Arc::new(|| {
                    if let Err(error) = escher_os::dialog::alert("Anvil", "Hello from a real native menu action!") {
                        tracing::warn!("Demo menu alert failed: {error}");
                    }
                }),
            },
            MenuItem::Item {
                label: "Play Sound".to_string(),
                key_equivalent: String::new(),
                action: Arc::new(|| {
                    if let Err(error) = escher_os::sound::play("Glass") {
                        tracing::warn!("Demo menu sound failed: {error}");
                    }
                }),
            },
            MenuItem::Item {
                label: "Copy Debug Info".to_string(),
                key_equivalent: String::new(),
                action: Arc::new(|| {
                    let info = format!("Anvil pid {}", std::process::id());
                    if let Err(error) = escher_os::clipboard::write_text(&info) {
                        tracing::warn!("Demo menu clipboard copy failed: {error}");
                    }
                }),
            },
        ],
    }
}

/// A `url`'s host, for favicon lookup/display — plain string splitting, not a real URL parser
/// (no crate in this workspace already pulls one in for something this small); good enough for
/// the `scheme://host/path` shape every URL typed into the address bar actually has.
fn host_of(url: &str) -> String {
    url.split("://").nth(1).and_then(|rest| rest.split('/').next()).unwrap_or(url).to_string()
}

/// Every tab's own `WebView`, keyed by `Tab::id` — not `escher_bevy::webview::WebViewHandles`
/// (that's one-per-*window*, the right shape for the old one-OS-window-per-scene design, wrong
/// for one window hosting many tabs). `NonSend` for the same reason `WebViewHandles` is: a
/// `WebView` wraps a native AppKit object.
#[derive(Default)]
struct TabWebViews(std::collections::HashMap<u64, escher_webview::WebView>);

/// A stub `anvil://` page — registered on every tab's webview, so `anvil://settings` works exactly
/// like any other URL typed into the address bar or navigated to in code. Built as a real
/// `escher_core::scaffold::Scaffold` via the same `style`/`slot`/`content` builder
/// every native Escher surface already composes with (see `packages/chalk/src/toolbar.rs` for the
/// same pattern), rendered via `escher_web::ssg::render_scaffold_to_html` — not a
/// `ScaffoldDescription` hand-built in Rust (an earlier version of this function did exactly
/// that — that type only exists to cross the wire from a JSX-authoring tool; constructing one by
/// hand in app code bypasses Escher's own UI composition patterns rather than using them). Colors
/// match `spec/design/styleguide/anvil.md`'s palette by literal value, not by reading the file —
/// `escher-web` has no reason to depend on `escher-styleguide` for one static page — and the
/// content itself is deliberately just a functional stub, expected to only need style tweaks
/// from here, not more plumbing.
fn anvil_scheme_handler() -> escher_webview::CustomSchemeHandler {
    use escher_core::draw::Bump;
    use escher_core::scaffold::Scaffold;
    use escher_core::style::BackgroundColor;
    use escher_core::style::ContentColor;
    use escher_core::style::FlexDirection;
    use escher_core::style::Gap;
    use escher_core::style::Padding;
    use escher_core::style::Value;

    struct Title;
    struct Body;

    escher_webview::CustomSchemeHandler {
        scheme: "anvil".to_string(),
        handler: Arc::new(|url: &str| {
            let page = url.strip_prefix("anvil://").unwrap_or(url);
            match page.trim_end_matches('/') {
                "settings" => {
                    let arena = Bump::new();
                    let root = Scaffold::new_in(&arena)
                        .style(FlexDirection::Column)
                        .style(Padding::all(32))
                        .style(Gap(Value::from(12)))
                        .style(BackgroundColor::try_from("#1a1b26").unwrap_or_default())
                        .slot::<Title>(|title| {
                            title.style(ContentColor::try_from("#7aa2f7").unwrap_or_default()).content(Some("Anvil Settings".to_string()))
                        })
                        .slot::<Body>(|body| {
                            body.style(ContentColor::try_from("#565f89").unwrap_or_default())
                                .content(Some("Nothing configurable here yet — this is a stub.".to_string()))
                        });

                    Some(escher_web::ssg::render_scaffold_to_html(&root))
                }
                _ => None,
            }
        }),
    }
}

/// Builds the extra items every tab's webview offers on a link's right-click context menu —
/// "Open Link in New Tab" (reusing the exact same `pending_browser_urls` queue + wake `/browser`
/// already uses, so this is indistinguishable, plumbing-wise, from typing `/browser <url>`) and
/// "Copy Link Address" (`escher_os::clipboard`, already correctly reusable anywhere). Returns a
/// `Vec`, not a single fixed action, deliberately — this is the extension point for pane-
/// management-flavored actions later ("open to the right/left/top/bottom"), once that exists:
/// adding one is appending another `ContextMenuItem` here, not touching `escher-webview`'s own
/// context-menu plumbing again.
fn link_context_menu_items(
    pending_browser_urls: Arc<Mutex<Vec<String>>>,
    event_loop_proxy: bevy::winit::EventLoopProxy<bevy::winit::WinitUserEvent>,
) -> impl Fn(&str) -> Vec<escher_webview::ContextMenuItem> {
    move |url: &str| {
        let url = url.to_string();

        let open_in_new_tab = {
            let url = url.clone();
            let pending_browser_urls = pending_browser_urls.clone();
            let event_loop_proxy = event_loop_proxy.clone();
            escher_webview::ContextMenuItem {
                label: "Open Link in New Tab".to_string(),
                action: Arc::new(move || {
                    pending_browser_urls.lock().push(url.clone());
                    let _ = event_loop_proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
                }),
            }
        };

        let copy_link = escher_webview::ContextMenuItem {
            label: "Copy Link Address".to_string(),
            action: Arc::new(move || {
                if let Err(error) = escher_os::clipboard::write_text(&url) {
                    tracing::warn!("Failed to copy link address: {error}");
                }
            }),
        };

        vec![open_in_new_tab, copy_link]
    }
}

/// Attaches a `WebView` for any tab that doesn't have one yet — runs every tick, picking up tabs
/// added by `open_tab` (via `/browser` or the "+ New Tab" button) the moment the browser window's
/// native handle exists. Every tab's webview shares the same insets (toolbar height, tab strip
/// width); only the active one is left visible, so a newly-opened tab starts hidden unless it's
/// also the one that just became active.
///
/// Waits one full tick before actually attaching (`Tab::attach_deferred`): `apply_browser_
/// navigation` (adding the tab to `browser.tabs`) and this system both run within the same
/// `Update` stage, i.e. the same frame — attaching immediately would pay `WebView::attach`'s
/// synchronous native-view-creation cost (a real, measurable hitch, worse on the very first
/// webview a process creates) *before* that frame ever gets presented, so the new tab row and its
/// `loading` state never actually reach the screen until the attach is already done. Deferring one
/// tick lets that frame present first — the tab row appears, showing `loading`, instantly — and
/// pays the attach cost on the next one instead.
///
/// That deferred tick has to be explicitly woken (`event_loop_proxy`), not just assumed to
/// happen "next frame": under `WinitSettings::desktop_app()` (see `EscherBevyPlugin`), `Update`
/// only runs again once a real winit event arrives — with nothing here to prompt one, the actual
/// attach wouldn't run until the user happened to move the mouse or the 5s/60s idle fallback fired,
/// which is exactly the "still feels clunky" symptom this was meant to fix, just moved one step
/// later. Same fix shape `surface.rs` already uses for click responsiveness (see its own comments
/// citing this same idle-fallback timer).
fn attach_pending_tab_webviews(
    mut webviews: NonSendMut<TabWebViews>,
    mut browser: ResMut<BrowserState>,
    tab_strip: Res<TabStripState>,
    window_query: Query<&RawHandleWrapper>,
    event_loop_proxy: Res<bevy::winit::EventLoopProxyWrapper>,
    state: Res<AppState>,
) {
    let Some(window) = browser.window else { return };
    let Ok(raw_handle) = window_query.get(window) else { return };
    let active = browser.active;

    for tab in &mut browser.tabs {
        if webviews.0.contains_key(&tab.id) {
            continue;
        }

        if !tab.attach_deferred {
            tab.attach_deferred = true;
            let proxy: bevy::winit::EventLoopProxy<bevy::winit::WinitUserEvent> = Clone::clone(&event_loop_proxy);
            let _ = proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
            continue;
        }

        match escher_webview::WebView::attach(
            raw_handle.get_window_handle(),
            &tab.url,
            TOOLBAR_HEIGHT,
            // See `tab_strip_content_inset`'s own doc comment — a fresh webview's initial inset
            // needs this too, not just the two places that update it later.
            tab_strip_content_inset(&tab_strip),
            Some(escher_webview::DEFAULT_USER_AGENT),
            link_context_menu_items(state.pending_browser_urls.clone(), Clone::clone(&event_loop_proxy)),
            Some(anvil_scheme_handler()),
        ) {
            Ok(webview) => {
                webview.set_hidden(active != Some(tab.id));
                tracing::info!("Opened tab: {}", tab.url);
                webviews.0.insert(tab.id, webview);
            }
            Err(error) => tracing::warn!("Failed to attach webview for '{}': {error}", tab.url),
        }
    }
}

/// Refreshes every tab's own `loading` flag from its `WebView::is_loading()` — nav state lives on
/// the `Tab` itself, not recomputed ad hoc wherever something needs it (`sync_toolbar_state`
/// used to reach into `TabWebViews` directly just for the active tab; now every tab's own state
/// stays current, which the tab strip can also draw from later for background-tab indicators).
/// Leaves `loading` untouched (rather than forcing `false`) for a tab with no `WebView` yet — see
/// `Tab::loading`'s own doc comment: `open_tab` already starts it `true`, and this system has
/// nothing truthful to say about a tab that hasn't attached its webview yet, so it just doesn't
/// contradict that until there's a real answer to report.
fn sync_tab_loading_state(mut browser: ResMut<BrowserState>, webviews: NonSend<TabWebViews>) {
    for tab in &mut browser.tabs {
        if let Some(webview) = webviews.0.get(&tab.id) {
            tab.loading = webview.is_loading();
        }
    }
}

/// Refreshes every tab's own `title` from its `WebView::title()` — previously never touched past
/// tab-creation time (`open_tab` sets it to the URL's own host as a placeholder, and nothing ever
/// updated it afterward), so every tab's label stayed frozen on its hostname forever regardless of
/// what page actually loaded, real browsers all reflect the page's own `<title>` once one loads.
/// Falls back to the host placeholder for a page that hasn't set a title yet (`unwrap_or_else`),
/// same reasoning as `sync_toolbar_state`'s own window-title fallback.
fn sync_tab_titles(mut browser: ResMut<BrowserState>, webviews: NonSend<TabWebViews>) {
    for tab in &mut browser.tabs {
        if let Some(webview) = webviews.0.get(&tab.id) {
            tab.title = webview.title().filter(|title| !title.is_empty()).unwrap_or_else(|| tab.host.clone());
        }
    }
}

/// Pushes `BrowserState`'s current shape into `ToolbarState`/`TabStripState` every tick, just
/// before `ToolbarSystems` redraws from them — the one place anvil's own tab bookkeeping and
/// `escher_appkit::bevy`'s neutral display state meet.
fn sync_toolbar_state(browser: Res<BrowserState>, mut toolbar: ResMut<ToolbarState>, mut tab_strip: ResMut<TabStripState>, mut windows: Query<&mut Window>) {
    toolbar.address = browser.active_tab().map(|tab| tab.url.clone()).unwrap_or_default();
    toolbar.loading = browser.active_tab().map(|tab| tab.loading).unwrap_or(false);
    tab_strip.tabs = browser.tabs.iter().map(|tab| TabInfo { id: tab.id, title: tab.title.clone(), host: tab.host.clone() }).collect();
    tab_strip.active = browser.active;

    // Every other real browser reflects the active page's own title in the native window title
    // bar (Edge, the YouTube PWA used as this session's design reference, ...) — Anvil's window
    // was hardcoded to a static "Anvil — Browser" at creation and never touched again. Falls
    // back to that same static title with no tabs open (`unwrap_or_else`), rather than an empty
    // bar. Guarded on an actual change: `Window` is a plain component, so writing through
    // `&mut Window` every tick regardless would mark it changed every tick, which is wasted work
    // downstream for anything else watching for a real title change.
    let title = browser.active_tab().map(|tab| tab.title.clone()).unwrap_or_else(|| "Anvil — Browser".to_string());
    if let Some(window_entity) = browser.window
        && let Ok(mut window) = windows.get_mut(window_entity)
        && window.title != title
    {
        window.title = title;
    }
}

/// Hides every tab's webview except `active`'s — the whole mechanism behind "switching tabs" now
/// that each tab has its own `WebView` (see `TabWebViews`): no reload, just a visibility flip, so
/// whatever the previously-hidden tab had on screen (scroll position, form state, JS state) is
/// still there next time it's shown. A newly-opened tab that hasn't been attached yet is handled
/// separately, by `attach_pending_tab_webviews` checking `BrowserState::active` at attach time.
fn show_only(webviews: &TabWebViews, active: Option<u64>) {
    for (id, webview) in &webviews.0 {
        webview.set_hidden(Some(*id) != active);
    }
}

/// Consumes `ToolbarEvent`/`TabStripEvent` (emitted this same tick by `ToolbarSystems`, so a click
/// takes effect on the very next redraw, not one tick later) and applies them to `BrowserState`/
/// `TabWebViews`/`TabStripState` — the one place all three actually get mutated.
fn apply_browser_navigation(
    mut browser: ResMut<BrowserState>,
    mut webviews: NonSendMut<TabWebViews>,
    mut tab_strip: ResMut<TabStripState>,
    mut toolbar: ResMut<ToolbarState>,
    mut toolbar_events: MessageReader<ToolbarEvent>,
    mut tab_events: MessageReader<TabStripEvent>,
    mut windows: Query<&mut bevy::window::Window>,
    state: Res<AppState>,
) {
    for event in toolbar_events.read() {
        match event {
            ToolbarEvent::Back => {
                if let Some(id) = browser.active
                    && let Some(webview) = webviews.0.get(&id)
                {
                    webview.go_back();
                }
            }
            ToolbarEvent::Forward => {
                if let Some(id) = browser.active
                    && let Some(webview) = webviews.0.get(&id)
                {
                    webview.go_forward();
                }
            }
            ToolbarEvent::Refresh => {
                if let Some(id) = browser.active
                    && let Some(webview) = webviews.0.get(&id)
                    && let Some(tab) = browser.active_tab()
                {
                    let _ = webview.load(&tab.url);
                }
            }
            ToolbarEvent::Navigate(url) => {
                let host = host_of(url);
                let active = browser.active;
                if let Some(tab) = browser.active_tab_mut() {
                    tab.url = url.clone();
                    tab.title = host.clone();
                    tab.host = host;
                }
                if let Some(id) = active
                    && let Some(webview) = webviews.0.get(&id)
                {
                    let _ = webview.load(url);
                }
            }
            ToolbarEvent::ToggleSidebar => {
                tab_strip.toggle_collapsed();
                // See `tab_strip_content_inset`'s own doc comment.
                let width = tab_strip_content_inset(&tab_strip);
                for webview in webviews.0.values() {
                    webview.set_left_inset(width);
                }
                persist_sidebar_state(&tab_strip, &state);
            }
            ToolbarEvent::TogglePinned => {
                toolbar.pinned = !toolbar.pinned;
                let level = if toolbar.pinned { bevy::window::WindowLevel::AlwaysOnTop } else { bevy::window::WindowLevel::Normal };
                if let Some(id) = browser.window
                    && let Ok(mut window) = windows.get_mut(id)
                {
                    window.window_level = level;
                }
            }
        }
    }

    for event in tab_events.read() {
        match event {
            TabStripEvent::Select(id) => {
                browser.active = Some(*id);
                show_only(&webviews, browser.active);
            }
            TabStripEvent::Close(id) => {
                let closing_active = browser.active == Some(*id);
                browser.tabs.retain(|tab| tab.id != *id);
                webviews.0.remove(id);
                if closing_active {
                    browser.active = browser.tabs.first().map(|tab| tab.id);
                    show_only(&webviews, browser.active);
                }
            }
            TabStripEvent::Reorder(id, positions) => {
                if let Some(index) = browser.tabs.iter().position(|tab| tab.id == *id) {
                    let new_index = (index as i32 + positions).clamp(0, browser.tabs.len() as i32 - 1) as usize;
                    if new_index != index {
                        let tab = browser.tabs.remove(index);
                        browser.tabs.insert(new_index, tab);
                    }
                }
            }
            TabStripEvent::New => {
                open_tab(&mut browser, "https://www.google.com".to_string());
                show_only(&webviews, browser.active);
            }
            TabStripEvent::Resize(delta) => {
                tab_strip.width = (tab_strip.width + delta).clamp(MIN_WIDTH, MAX_WIDTH);
                if tab_strip.width >= ICON_ONLY_WIDTH {
                    tab_strip.expanded_width = tab_strip.width;
                }
                persist_sidebar_state(&tab_strip, &state);
                // See `tab_strip_content_inset`'s own doc comment.
                let width = tab_strip_content_inset(&tab_strip);
                for webview in webviews.0.values() {
                    webview.set_left_inset(width);
                }
            }
        }
    }
}

/// Mirrors `tab_strip`'s current `(width, expanded_width)` into `AppState::sidebar_state` and
/// queues a `PersistenceWrite::SidebarState` — called from both `ToolbarEvent::ToggleSidebar` and
/// `TabStripEvent::Resize`, the only two places `TabStripState`'s width-related fields ever
/// change. Same "never block the render thread, a failed save is just logged" tradeoff as every
/// other persistence call site (see `AppState::persistence_writes`'s doc comment).
fn persist_sidebar_state(tab_strip: &TabStripState, state: &AppState) {
    *state.sidebar_state.write() = (tab_strip.width, tab_strip.expanded_width);
    if let Some(sender) = state.persistence_writes.read().clone() {
        let _ = sender.send(PersistenceWrite::SidebarState(tab_strip.width, tab_strip.expanded_width));
    }
}

fn open_tab(browser: &mut BrowserState, url: String) {
    let id = browser.next_id;
    browser.next_id += 1;
    let host = host_of(&url);
    // `loading: true` from the start — see `Tab::loading`'s own doc comment — and
    // `attach_deferred: false` so `attach_pending_tab_webviews` gives this tab one full frame to
    // actually appear in the tab strip before doing the slow part.
    browser.tabs.push(Tab { id, url: url.clone(), title: host.clone(), host, loading: true, attach_deferred: false });
    browser.active = Some(id);
}

/// Resets `BrowserState` back to "no window open" the moment the browser window's entity stops
/// existing (the user closed it — Bevy despawns the entity itself, there's no separate "closed"
/// flag to read). Without this, `browser.window` keeps pointing at a dead `Entity` forever: every
/// later `/browser` call takes the "add a tab to the existing window" branch, `attach_pending_tab_
/// webviews`'s entity lookup fails silently, and nothing visibly happens — no error, no new
/// window, no tab. Must run before `spawn_browser_window_on_command` each tick so a `/browser`
/// right after a close creates a fresh window instead of one more silent no-op.
fn clear_browser_state_on_window_close(mut browser: ResMut<BrowserState>, mut webviews: NonSendMut<TabWebViews>, window_query: Query<Entity, With<bevy::window::Window>>) {
    let Some(window) = browser.window else { return };
    if window_query.get(window).is_err() {
        tracing::info!("Browser window closed; resetting tab state");
        *browser = BrowserState::default();
        webviews.0.clear();
    }
}

/// `/browser <url>` opens (or focuses) the single browser window instead of a brand new OS window
/// per call — the tabbed-browser redesign superseding the earlier one-window-per-scene approach.
/// First call creates the window (webview + toolbar + tab strip, `WindowLevel::AlwaysOnTop` iff
/// `AppState::always_on_top`, live-toggleable afterward via the toolbar's own pin button — see
/// `ToolbarEvent::TogglePinned`); every later call just opens a new tab in it.
fn spawn_browser_window_on_command(
    mut commands: Commands,
    mut browser_evt: MessageReader<SceneCommand>,
    mut browser: ResMut<BrowserState>,
    mut tab_strip: ResMut<TabStripState>,
    mut toolbar: ResMut<ToolbarState>,
    webviews: NonSend<TabWebViews>,
    state: Res<AppState>,
) {
    let window_level = if state.always_on_top { bevy::window::WindowLevel::AlwaysOnTop } else { bevy::window::WindowLevel::Normal };

    for SceneCommand { url } in browser_evt.read() {
        if browser.window.is_none() {
            toolbar.pinned = state.always_on_top;
            let mut window = escher_bevy::window::create_window("Anvil — Browser", 1100.0, 760.0, true, window_level);
            // Lets the toolbar (`WantsToolbar`'s `Pin::Top` surface, see `AppKitSurface::attach`)
            // paint underneath the native titlebar instead of below it — `fullsize_content_view`
            // extends the content view up into the titlebar's own area, `titlebar_transparent`
            // stops that area painting the usual opaque titlebar material over it, and
            // `titlebar_show_title` off drops the redundant centered window title now that the
            // toolbar itself occupies that space. Traffic-light buttons stay put (AppKit floats
            // them above the content view regardless) — `escher_chalk::toolbar::toolbar`'s own
            // leading padding reserves room for them.
            window.titlebar_transparent = true;
            window.fullsize_content_view = true;
            window.titlebar_show_title = false;

            let window_entity = commands.spawn((window, WantsToolbar, WantsTabStrip, FocusPending)).id();

            // Seeds the (global, singleton) `TabStripState` from whatever was last saved — safe
            // to do unconditionally here since this branch only ever runs once, the first time
            // the single browser window is created (see `browser.window.is_none()` above).
            let (width, expanded_width) = *state.sidebar_state.read();
            tab_strip.width = width;
            tab_strip.expanded_width = expanded_width;

            browser.window = Some(window_entity);
            open_tab(&mut browser, url.clone());
            // `attach_pending_tab_webviews` picks this tab up the moment the window's native
            // handle exists (opts into `escher_webview::DEFAULT_USER_AGENT` for every tab —
            // Anvil's browser window is general-purpose browsing, Google/YouTube included, where a
            // real desktop-Safari UA is the right default; see that constant's own doc comment).

            // The single browser window needs its own camera targeting it — there's no primary
            // window for a default-target camera to fall back to (see `main`'s
            // `spawn_primary_window(false)`). This is what paints the black `ClearColor` behind
            // the webview's own native view.
            commands.spawn((Camera2d, bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window_entity))));
        } else {
            open_tab(&mut browser, url.clone());
            show_only(&webviews, browser.active);
            // The window already exists (and already has a real native handle), so
            // `focus_new_windows` picks this back up on the very next tick and calls
            // `focus_window()` immediately — same mechanism a brand-new window gets above, just
            // re-armed here since opening another tab in an already-open window previously left
            // it wherever it already was in z-order (behind other apps, easy to miss).
            if let Some(window_entity) = browser.window {
                commands.entity(window_entity).insert(FocusPending);
            }
        }
    }
}

/// `/scene` opens a plain Bevy-rendered window — a stub, not yet the real scene-inspection view
/// it's meant to grow into. A distinct tinted background plus a label spelling out "this is a
/// stub" distinguish an intentionally empty scene from a crashed/blank one; without either, the
/// window looked broken since default `ClearColorConfig` renders near-black with nothing on it.
/// Checks and clears `AppState::scene_window_requested` each tick rather than a `Message`: a bare
/// `/scene` carries no data to queue, so there's nothing a `Message` payload would add over a
/// flag.
fn spawn_scene_window_on_command(mut commands: Commands, state: Res<AppState>) {
    if !state.scene_window_requested.swap(false, Ordering::Relaxed) {
        return;
    }

    // No toolbar/pin button on this window (a stub, see this function's own doc comment) — just
    // `AppState::always_on_top`'s starting value, with no live per-window toggle available yet.
    let window_level = if state.always_on_top { bevy::window::WindowLevel::AlwaysOnTop } else { bevy::window::WindowLevel::Normal };

    let window_entity = commands.spawn((escher_bevy::window::create_window("Anvil — Scene", 800.0, 600.0, true, window_level), FocusPending)).id();
    commands.spawn((
        Camera2d,
        bevy::camera::Camera { clear_color: ClearColorConfig::Custom(BevyColor::hsla(220.0, 0.15, 0.16, 1.0)), ..Default::default() },
        bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window_entity)),
    ));
    commands.spawn((
        Text2d::new("Empty scene (/scene is a stub — nothing spawns here yet)"),
        TextFont { font_size: 18.0, ..Default::default() },
        TextColor(BevyColor::hsla(220.0, 0.1, 0.6, 1.0)),
    ));
}

/// Marks a just-spawned browser/scene window entity so [`focus_new_windows`] brings it to the
/// front exactly once. Regardless of `AppState::always_on_top` (whether a
/// window keeps floating above everything afterward), a *newly opened* window should always be
/// the one you see — a background app creating a new `WindowLevel::Normal` window doesn't
/// necessarily steal focus/front-ordering from whatever was already active, which read as "the
/// command did nothing" when the new window landed behind it with zero indication.
#[derive(Component)]
pub struct FocusPending;

/// Removes `FocusPending` the moment a window's real native handle exists and asks winit to focus
/// it — same one-tick-deferral shape `attach_pending_tab_webviews` already uses for the same
/// reason (the handle isn't available the very first frame).
fn focus_new_windows(
    mut commands: Commands,
    windows: Query<Entity, (With<FocusPending>, With<RawHandleWrapper>)>,
    winit_windows: Option<NonSend<bevy::winit::WinitWindows>>,
) {
    // `WinitWindows` isn't inserted at all until `bevy_winit` has actually managed a real
    // window. Anvil starts with zero windows (`spawn_primary_window(false)`), so this resource
    // genuinely doesn't exist yet on every
    // tick before the first `/browser`/`/scene` command, and nothing in this query being ready
    // yet doesn't mean the resource itself exists to even ask.
    let Some(winit_windows) = winit_windows else { return };

    for entity in windows.iter() {
        if let Some(window) = winit_windows.get_window(entity) {
            window.focus_window();
            // `focus_window()` alone can reorder this window among Anvil's *own* windows
            // without actually stealing focus from another app entirely (confirmed live: a
            // `/relay-console`/`/browser` reopening an existing tab left the browser window
            // behind the terminal emulator Anvil was launched from) — a raw binary launched
            // from a terminal isn't always treated as "the active app" the way a real, Dock-
            // launched `.app` bundle is. `escher_os::activation::activate` is the app-level half
            // this window-level call needs alongside it.
            let _ = escher_os::activation::activate();
        }
        commands.entity(entity).remove::<FocusPending>();
    }
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();

    // Standalone, no tracing/persistence/TUI setup needed — see `config::run_init`'s own doc
    // comment.
    if let Some(Command::Init { sqld_url, ollama_url }) = args.command {
        config::run_init(sqld_url, ollama_url);
        return Ok(ExitCode::SUCCESS);
    }

    // `.anvil.toml` (see `config.rs`) is the lowest-priority source for both addresses below —
    // `--connect`/`ATLAS_SYNC_URL` still win over its `sqld.url` if given, matching how an
    // explicit flag/env var should always beat a project-directory default. Loaded before
    // `AppState::new`/anything spawns a JS command, since both need to already see the resolved
    // value.
    let project_config = config::AnvilConfig::load_from_cwd();
    if let Some(ollama_url) = project_config.as_ref().and_then(|config| config.ollama.as_ref()) {
        // SAFETY: called at the very top of `main`, before any other thread (the tokio runtime
        // spawned below included) exists to race this write.
        unsafe { std::env::set_var("ANVIL_OLLAMA_URL", &ollama_url.url) };
    }
    // `--always-on-top` wins over `.anvil.toml`'s `[window] always_on_top`, same precedence as
    // the addresses above — just this instance's *starting* value, the toolbar's own pin button
    // (see `ToolbarEvent::TogglePinned`) can still change it live afterward.
    let always_on_top =
        args.always_on_top || project_config.as_ref().and_then(|config| config.window.as_ref()).is_some_and(|window| window.always_on_top);
    // `.anvil.toml`'s `[welcome]` table lets a project override the new-user tagline and the
    // palette's usage-note footer without editing Rust.
    let welcome_config = project_config.as_ref().and_then(|config| config.welcome.as_ref());
    let welcome_tagline = welcome_config.and_then(|welcome| welcome.tagline.clone()).unwrap_or_else(|| WELCOME_TAGLINE.to_string());
    let welcome_footer = welcome_config.and_then(|welcome| welcome.footer.clone()).unwrap_or_else(|| DEFAULT_WELCOME_FOOTER.to_string());

    // `--connect` first, then `ATLAS_SYNC_URL` (the same env var Atlas's own `examples/sync`
    // already reads for exactly this — reusing it rather than inventing an Anvil-specific one),
    // then `.anvil.toml`'s `sqld.url`, then `persistence::DEFAULT_SQLD_URL`. `None` here means
    // "use the default."
    let sqld_url = args
        .connect
        .clone()
        .or_else(|| std::env::var(atlas::env::SYNC_URL_KEY).ok())
        .or_else(|| project_config.as_ref().and_then(|config| config.sqld.as_ref()).map(|sqld| sqld.url.clone()));
    let identity = args.identity.clone().unwrap_or_else(|| format!("anvil-{}", std::process::id()));
    // A real, fixed-size UUID is what actually gets persisted (see `ANVIL_IDENTITY_NAMESPACE`'s
    // own doc comment) — `identity` above stays the human-facing label (shown in `Page::Inspect`,
    // passed on the command line), this is purely a derived storage key.
    let identity_uuid = uuid::Uuid::new_v5(&ANVIL_IDENTITY_NAMESPACE, identity.as_bytes());

    color_eyre::install()?;

    // `color_eyre`'s own panic hook only writes to stderr, which for a raw-mode/alternate-screen
    // terminal app is not a reliable place for a message to survive — the alt screen is torn down
    // (or the process aborts) before anyone can read it. Chaining a second hook that also appends
    // the panic message and a backtrace to a plain file gives a real crash trail independent of
    // whatever the terminal itself was doing when the panic happened.
    let default_panic_hook = std::panic::take_hook();
    let panic_log_path = anvil_log_dir().join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&panic_log_path) {
            let _ = writeln!(file, "{info}\n{}", std::backtrace::Backtrace::force_capture());
        }
        default_panic_hook(info);
    }));

    // Every `tracing::*` call, from anywhere in the process, gets routed to a log file instead
    // of stdout — not just for this thread. A thread-local override (`with_default`, tried in
    // two earlier passes) can't reach everywhere a call might originate once persistence is in
    // the picture: libsql's async networking runs as its own tokio tasks, and its local
    // SQLite/WAL work runs on tokio's *blocking* thread pool — a separate pool that exists
    // regardless of runtime flavor. Both fall outside any thread-local scoping. Since the
    // terminal is in raw mode for nearly this whole run, any stray *printed* line corrupts the
    // screen — redirecting the global default itself is the only thing that covers every
    // thread uniformly. `RUST_LOG`/`--log-level` still controls verbosity; `tail -f` this
    // session's own log (see `anvil_log_dir`) to watch it live.
    //
    // Lives in this session's own pid-keyed directory (see `anvil_log_dir`) so two instances
    // running from the same folder don't truncate each other's log (`File::create` truncates on
    // open) out from under one another.
    let log_path = anvil_log_dir().join("anvil.log");
    let log_file = std::fs::File::create(&log_path)?;
    let file_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(false)
        .with_file(false)
        .with_ansi(false) // this is a plain log file now, not a terminal
        .with_writer(log_file)
        .without_time();

    // Everything logged inside a `live_trace` span (see `spawn_js_command`) also gets forwarded
    // here, live, so it reaches the transcript instead of only ever sitting in `anvil.log`
    // until `--dump-trace` prints it after the app has already exited.
    let (trace_tx, trace_rx) = mpsc::channel::<String>();
    let transcript_layer = escher_terminal::tracing_bridge::LiveTraceLayer::new(trace_tx);

    // Everything logged anywhere in the process — the same unscoped firehose `file_layer` above
    // writes to `anvil.log` — also lands here, live, in a bounded ring buffer. Backs
    // `Page::Trace` (toggled with F2 in `draw_assistant`): a way to see that raw feed without
    // leaving the app or tailing the log file in a second terminal. `.with_ansi(true)` is forced
    // rather than left to auto-detect, since `LineBuffer` as a `MakeWriter` isn't a real tty —
    // auto-detection would otherwise decide color should be off and strip it before it ever
    // reaches `LineBufferWriter`.
    // These run for the whole process lifetime (`trace_buffer` fed by every single `tracing::*`
    // call, including chatty libsql/sqld internals; `process_buffer` fed by every subprocess run
    // for the rest of the session), so an unbounded buffer would leak memory over a long session.
    const LINE_BUFFER_CAPACITY: usize = 2000;

    let trace_buffer = escher_terminal::tracing_bridge::LineBuffer::new(LINE_BUFFER_CAPACITY);
    let trace_page_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_writer(trace_buffer.clone());

    // A raw subprocess stdio feed — `Page::Process`, toggled with F3 — fed directly by
    // `run_js_command`, not through `tracing` at all (see `LineBuffer`'s own doc comment for why
    // it's a separate buffer from `trace_buffer` above, not just another `tracing` layer).
    let process_buffer = escher_terminal::tracing_bridge::LineBuffer::new(LINE_BUFFER_CAPACITY);

    // Shared with `AppState::raw_stream` below — the F1 handler in `draw_assistant` and
    // `--no-tui`'s startup value both write it, `RawStreamGate` (this layer's writer) and
    // `assistant_terminal_draw`'s own raw poll loop both read it. Starts at `args.no_tui` so
    // `--no-tui` skips the TUI from the very first frame instead of flashing it briefly first.
    let raw_stream_flag = Arc::new(AtomicBool::new(args.no_tui));
    let raw_stream_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(false)
        .with_file(false)
        .without_time()
        .with_writer(escher_terminal::tracing_bridge::RawStreamGate::new(raw_stream_flag.clone()));

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&args.log_level))
        .with(file_layer)
        .with(transcript_layer)
        .with(trace_page_layer)
        .with(raw_stream_layer)
        .init();

    //--
    // Multi-threaded: `AppState::new`'s background persistence connect (`spawn_connect_persistence`)
    // needs genuine progress independent of the synchronous render loop's own occasional
    // `block_on` calls. A current-thread runtime only polls spawned tasks while something is
    // actively blocked on it, which the render loop isn't doing continuously.
    let runtime = Arc::new(tokio::runtime::Builder::new_multi_thread().enable_all().build()?);

    if args.reset_data {
        // Plain stdout, no TUI — the whole point is a quick one-shot cleanup, not another raw-
        // mode session.
        return runtime.block_on(async {
            let persistence = persistence::Persistence::connect(sqld_url.as_deref()).await?;
            persistence.reset().await?;
            println!("Cleared all persisted messages and tasks.");
            Ok(ExitCode::SUCCESS)
        });
    }

    let app_state = runtime.block_on(AppState::new(
        runtime.clone(),
        trace_rx,
        trace_buffer,
        process_buffer,
        raw_stream_flag.clone(),
        sqld_url,
        identity,
        identity_uuid,
        always_on_top,
        welcome_tagline,
        welcome_footer,
    ));

    // "Relay console, escher, etc" should always be reachable from a `*.localhost` URL for as
    // long as Anvil is running, per the user directly — not lazily started the first time
    // `/relay-console`/a browser tab asks for one. Both of `spawn_relay_console_server`/
    // `spawn_relay_server` are already idempotent (an atomic "already started" guard each), so
    // a later `/relay-console` calling them again is a no-op, not a double-start.
    app_state.spawn_relay_console_server();
    app_state.spawn_relay_server();
    std::thread::spawn(|| config::ensure_docker_service_running("escher web", "http://127.0.0.1:3615", "web"));

    // `/browser` (the `Enter`-key handler further down) fires a real in-process `SceneCommand`
    // instead of spawning a second process for it — see `AssistantTerminalPlugin`'s own doc
    // comment for why that means this whole app is a Bevy app now. Every `SceneCommand` opens a
    // brand new, independent window (`spawn_browser_window_on_command`) — no single shared window
    // to pre-warm, so `spawn_primary_window(false)` means the app starts with none at all, until
    // the first `/browser` or `/scene`.
    //
    // `with_exit_condition(DontExit)`: a scene window's lifetime is independent of the terminal's
    // — closing one (the native close button, left at Bevy's own default handling) despawns just
    // that window, not the process, since the terminal UI this app shares a process with has to
    // keep running regardless of how many scene windows are currently open (including zero).
    // Without this, Bevy's own default (`OnAllClosed`) exits the whole process the moment the
    // *last* window closes — the terminal side already has its own real exit path (Ctrl+C/a
    // signal writing `AppExit`), that's the only thing that should end this process.
    App::new()
        .add_plugins(EscherBevyPlugin::new(
            EscherBevyConfig::default()
                .with_clear_color(BevyColor::BLACK)
                .with_window_title("Anvil")
                .with_spawn_primary_window(false)
                .with_exit_condition(bevy::window::ExitCondition::DontExit)
                // `AssistantTerminalPlugin` (below) is this app's own full terminal UI —
                // `escher-bevy`'s generic `terminal::TerminalPlugin` would otherwise also spawn
                // and race it for the same OS terminal (see `EscherBevyConfig::
                // spawn_terminal_plugin`'s own doc comment) — that race is the cause of a
                // garbled header and doubled per-frame redraw/input-poll work. The
                // `terminal` Cargo feature itself stays on regardless — this app still uses its
                // plain helper functions (`spawn_input_watcher` etc.), just not the plugin.
                .with_spawn_terminal_plugin(false),
        ))
        .add_plugins(OsPlugin::new("Anvil").with_extra_menu_items(vec![demo_menu()]))
        .add_plugins(ToolbarPlugin)
        .add_message::<SceneCommand>()
        .insert_non_send_resource(TabWebViews::default())
        .insert_resource(BrowserState::default())
        .insert_resource(TabStripState::default())
        .insert_resource(ThemeState(Some(ToolbarTheme {
            background: styleguide_color("background", (32, 32, 32)),
            chrome: styleguide_color("chrome", (38, 42, 48)),
            surface: styleguide_color("surface", (51, 56, 68)),
            control_hover: styleguide_color("control-hover", (61, 67, 80)),
            border: styleguide_color("border", (64, 69, 79)),
            accent: styleguide_color("accent", (97, 175, 239)),
            text: styleguide_color("text", (232, 232, 232)),
            ui_text_size: styleguide_text_size("ui", 15.0),
            body_text_size: styleguide_text_size("body", 13.0),
        })))
        .insert_resource(app_state)
        .add_plugins(AssistantTerminalPlugin)
        .add_systems(
            Update,
            (
                clear_browser_state_on_window_close,
                spawn_browser_window_on_command,
                spawn_scene_window_on_command,
                attach_pending_tab_webviews,
                sync_tab_loading_state,
                sync_tab_titles,
                sync_toolbar_state,
            )
                .chain()
                .before(ToolbarSystems),
        )
        .add_systems(Update, focus_new_windows)
        .add_systems(Update, apply_browser_navigation.after(ToolbarSystems))
        .run();

    //--
    tracing::info!("Bye! <3");

    // By this point `assistant_terminal_exit` has already left the alternate screen and disabled
    // raw mode, so printing straight to stdout is safe again — it won't corrupt anything the app
    // was drawing, because it isn't drawing anymore.
    if args.dump_trace {
        match std::fs::read_to_string(&log_path) {
            Ok(trace) => {
                println!("--- {} ---", log_path.display());
                print!("{trace}");
            }
            Err(error) => {
                eprintln!("Failed to read {} for --dump-trace: {error}", log_path.display());
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// A single turn in the transcript. Tool calls carry their own (fake) output, shown when expanded.
#[derive(Debug, Clone)]
enum ChatMessage {
    User(String),
    Assistant(String),
    Tool { name: String, detail: String, output: Vec<String> },
    /// A line forwarded live by `TranscriptLayer` while a command's `live_trace` span was
    /// active — ephemeral, not persisted (see `Persistence::save_message`), the same way you
    /// wouldn't expect last run's progress spam to still be there next time you open the app.
    Trace(String),
}

/// Posts a one-line assistant reply to the transcript — the common case of a command
/// acknowledging what it just did, in place of `messages.write().push(ChatMessage::Assistant(...))`
/// at every call site.
fn notify(messages: &RwLock<Vec<ChatMessage>>, text: impl Into<String>) {
    messages.write().push(ChatMessage::Assistant(text.into()));
}

/// Builds a `Vec<String>` from a mix of string literals, `owo_colors`-wrapped values, and other
/// `Display` expressions, each converted with a plain `.to_string()` — removes the
/// `format!("{}", ..)` noise of forcing one through the formatting machinery just to get a
/// `String` out, for the common case of writing a multi-line block (a `""` entry is a blank
/// line, no `String::new()` needed) like `AppState::welcome_overview_text`.
macro_rules! lines {
    ($($line:expr),* $(,)?) => {
        vec![$($line.to_string()),*]
    };
}

/// One entry in the "running tasks" overlay.
#[derive(Debug, Clone)]
struct TaskRow {
    label: String,
    status: String, // "done" | "running" | "pending"
}

/// The one thing a JS command's `run()` can return that means something other than "here's the
/// reply text to show" — see `commands/quit.js`. Deliberately just an exact-match plain string,
/// not a JSON envelope: every other JS command convention in this app returns plain text, and a
/// whole structured contract isn't worth it for the couple of cases (this, `CLEAR_SENTINEL`) that
/// need to signal something beyond that. A plain top-level `const`, not an associated one on
/// `AppState` — `persistence::is_hidden_from_history` needs it too, to keep `/quit`'s own
/// confirmation out of reloaded history (see that function's doc comment).
const QUIT_SENTINEL: &str = "💀";

/// Returned by `commands/clear.js` on success, in place of the plain `""` it used to return.
/// `""` alone only told `spawn_js_command` "don't record a reply" — it never touched `messages`
/// itself, so `/clear` wiped the persisted rows in `sqld` but left the live, in-memory transcript
/// (and the `/clear` invocation itself) sitting on screen exactly as before, looking like the
/// command silently did nothing. `spawn_js_command` now clears `messages` when it sees this exact
/// sentinel, same shape as `QUIT_SENTINEL` above. `/clear` stays a real JS command rather than a
/// Rust builtin per the user directly — as many commands as possible should live in scripts, to
/// build out that ecosystem, even ones (like this) that could be done natively.
const CLEAR_SENTINEL: &str = "🧹";

/// The fixed namespace `identity_uuid` (in `main`) hashes every instance's `--identity` string
/// against, via UUID v5 — a random UUID generated once for this app and hardcoded, per RFC 4122's
/// own scheme (any fixed, app-specific value works as a namespace; what matters is that it never
/// changes, since a different namespace would derive different UUIDs for the exact same identity
/// string). Not meant to be recognizable or looked up anywhere — it only exists so the same
/// `--identity "alice"` on two different machines always derives the exact same UUID.
const ANVIL_IDENTITY_NAMESPACE: uuid::Uuid = uuid::uuid!("06f116df-2aad-489f-990c-5711e1fababb");

/// Where this session's own on-disk state (the `sqld` replica cache, its log file) lives — a
/// session subdirectory under `~/.anvil` (or `$ANVIL_DATA_DIR`, if set) — state files belong
/// under the user's home directory, not scattered based on cwd — and this is
/// deliberately Anvil's own `~/.anvil`, not Atlas's `atlas::env::get_data_dir` convention, which is
/// Atlas's own isolated space for its own development data, kept separate from any specific app
/// that happens to depend on it.
///
/// Sessions are keyed by this process's own pid, so `anvil` run twice from the same folder gets
/// two sessions automatically — no flags or separate directories required (see
/// `persistence::connect_inner`'s identical reasoning for its replica file). Falls back to `.`
/// (this app's original cwd-relative behavior) if resolving/creating the real directory fails —
/// non-fatal, same as every other persistence-related failure in this file.
/// The directory this app's own scripts (`commands/`, `scripts/`) are resolved against — the
/// current working directory, full stop. No `CARGO_MANIFEST_DIR` fallback — per the user
/// directly, this needs to work independently of this dev checkout being present at all, and a
/// build-time path baked into the binary regardless would silently defeat that the moment
/// anything actually needed it. The binary installs to some arbitrary `PATH` location, but
/// `commands/` belongs to *the project being worked on* — `escher anvil` is meant to be run from
/// inside that project's own directory (the same one `escher init` scaffolds), so whatever real
/// commands exist there are what should run. A project with no `commands/` of its own simply has
/// none to discover — see `discover_js_commands`'s own handling of a missing directory.
/// `process::run_deno_command`/`run_reject_router` (their own `anvil_root`) and this file's own
/// `discover_js_commands`/relay-console-script lookups all go through this one function now, so
/// there's exactly one place deciding this, not one guess per call site that could disagree.
pub(crate) fn anvil_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// `/workspace`'s reply — a proof-of-embedding for `ethos_workspace::Workspace`, Anvil being its
/// first real consumer (see `ethos/spec/agents/proposals/workspace-core.md`). Scans `root` itself
/// for a `projects/` directory, so this only finds anything when Anvil's run from a real
/// Brainbow-style workspace root (`escher anvil --cwd .` from the monorepo root, not from
/// `apps/anvil` itself) — same "resolved purely from cwd, by design" convention `anvil_root`
/// already documents for `commands/`.
fn describe_workspace(root: &Path) -> String {
    let fs = ethos_workspace::NativeFs::new(root);
    let workspace = ethos_workspace::Workspace::scan(&fs, "");

    if workspace.projects.is_empty() {
        return format!("No `projects/` directory found under {} — Workspace found nothing to list.", root.display());
    }

    let mut lines = vec![format!("Workspace: {} project(s) under {}/projects", workspace.projects.len(), root.display())];
    for project in &workspace.projects {
        let mut markers = Vec::new();
        if project.kind.rust {
            markers.push("rust");
        }
        if project.kind.deno {
            markers.push("deno");
        }
        if project.kind.node {
            markers.push("node");
        }
        let markers = if markers.is_empty() { "unrecognized".to_string() } else { markers.join(", ") };
        lines.push(format!("- {} ({markers})", project.name));
    }
    lines.join("\n")
}

fn anvil_session_dir() -> PathBuf {
    let anvil_home = std::env::var("ANVIL_DATA_DIR")
        .map(PathBuf::from)
        .ok()
        .or_else(|| dirs::home_dir().map(|home| home.join(".anvil")))
        .unwrap_or_else(|| PathBuf::from("."));

    let session_dir = anvil_home.join("sessions").join(std::process::id().to_string());
    let _ = std::fs::create_dir_all(&session_dir);
    session_dir
}

/// Where this session's own log files (`anvil.log`, `panic.log`) live — `<project>/.output/
/// logs/<pid>/`, per the user directly: generated output belongs under a project's own
/// `.output/`, the same convention every other generated/build artifact in this monorepo
/// already follows (`runtimes/web/.output`, the root `.output/mdbook/`, ...), not scattered
/// under the user's home directory. Deliberately *not* `anvil_session_dir` — that one holds
/// real persisted state (the `sqld` replica cache) that genuinely needs a stable, cwd-
/// independent home; a log is disposable, project-scoped output, a different category of
/// thing. Still pid-keyed, same reasoning as `anvil_session_dir`'s own doc comment: two
/// instances running from the same project shouldn't truncate each other's log.
fn anvil_log_dir() -> PathBuf {
    let log_dir = anvil_root().join(".output").join("logs").join(std::process::id().to_string());
    let _ = std::fs::create_dir_all(&log_dir);
    log_dir
}

/// Persists the transcript and task list to a local libsql database that syncs against a
/// `sqld` server (`tools/data/compose.yaml` — `docker compose -f tools/data/compose.yaml up -d
/// sqld`), rather than just writing to a plain local SQLite file, per the human's request. This
/// is the "embedded replica" pattern: reads hit the local file directly (fast, works offline
/// once synced), writes are transparently forwarded to the remote `sqld` primary.
///
/// The terminal event loop (`TerminalApp::run`) is fully synchronous. There's no async executor
/// driving it, while `libsql`'s client is async-first. Bridged with one
/// `tokio::runtime::Runtime` created at startup: `.block_on()` at each call site for writes
/// (they only happen once per submitted message, so blocking briefly there is an acceptable
/// trade), and `.spawn()` for `AppState::new`'s startup connect, which needs to make real
/// progress in the background rather than blocking the TUI from appearing at all.

/// How long a single persistence write (`save_message`/`save_tasks`/`save_overlay_bounds`) is
/// allowed to block the render thread before giving up.
///
/// **This is the fix for the "moving/dragging/typing sometimes freezes for seconds to minutes"
/// bug**, found while diagnosing live user-reported symptoms. Every `store.save_*` call
/// site used to run via a bare `runtime.block_on(async { store.save_*(...).await })` with *no*
/// timeout at all — `persistence::CONNECT_TIMEOUT` only ever covered the initial connect, per its
/// own doc comment ("The connect/sync calls below have no timeout of their own"), and that
/// caveat turned out to apply to every later write too, not just those two calls. A slow or
/// wedged `sqld` (this repo has a documented history of that — see `spec/.agents/changelog.md`'s
/// "sqld crash loop" entry) would hang the *entire UI* — not just persistence — for
/// however long the underlying TCP call took to fail, which has no application-level bound and
/// can genuinely take minutes at the OS level. This matches the reported symptom exactly: it's
/// triggered by dragging the overlay (`sync_overlay_bounds_to_persistence`'s debounced save) and
/// by *any* input submission (every branch of the `Enter` handler below persists a `User` message
/// this same way), with inconsistent, unbounded duration depending on how `sqld` happens to be
/// behaving at that moment. Every one of those call sites now goes through
/// `block_on_with_timeout` instead, which enforces this bound. Also used to bound the periodic
/// resync loop's own `sync`/`load_*` calls (see `spawn_connect_persistence`) — those used to have
/// no timeout at all, unlike every save, so a stalled resync could hang not just that tick but,
/// transitively, the whole app's shutdown (dropping `AppState::runtime` blocks on any of its
/// still-running tasks finishing first).
const SAVE_TIMEOUT: Duration = Duration::from_millis(750);

/// Runs `future` (a single `store.save_*`/`load_*`/`sync` call), bounded by `SAVE_TIMEOUT` — see
/// its doc comment for why this exists. Flattens "the operation itself failed" and "it timed out"
/// into one `Result<T, String>`, since every call site only ever does `tracing::warn!` either way.
/// Async-native (`tokio::time::timeout`, no `runtime.block_on`) so it's safe to call from inside
/// an already-spawned task — a nested `block_on` on the same runtime would panic.
async fn with_sqld_timeout<F, T>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = color_eyre::Result<T>>,
{
    match tokio::time::timeout(SAVE_TIMEOUT, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err(format!("timed out after {SAVE_TIMEOUT:?} (sqld slow or unresponsive)")),
    }
}

/// One pending write for the single persistence-writer task (see `AppState::persistence_writes`
/// and `spawn_connect_persistence`'s writer loop) — `save_message`/`save_tasks`/
/// `save_overlay_bounds`, deferred until the writer's turn instead of racing straight into the
/// shared `Connection` from wherever the write originated.
enum PersistenceWrite {
    Message(ChatMessage),
    /// The *whole* current task list, not one changed row — see `Persistence::save_tasks`'s doc
    /// comment for why a single-row write isn't possible here. Coalesced the same way
    /// `OverlayBounds` is: if several land in one batch (e.g. creating a task right after cycling
    /// another's status), only the last snapshot actually needs to be written.
    Tasks(Vec<TaskRow>),
    OverlayBounds { bounds: (u16, u16, u16, u16), persisted_overlay_bounds: Arc<RwLock<Option<(u16, u16, u16, u16)>>> },
    /// `/welcome`'s toggle — a rare, deliberate write, not a stream, so unlike `Tasks`/
    /// `OverlayBounds` there's nothing to coalesce.
    ShowWelcomeOverview(bool),
    /// `(width, expanded_width)` — coalesced the same way `OverlayBounds` is: a live resize drag
    /// fires one of these per tick, and only the last snapshot in a batch is worth writing.
    SidebarState(f64, f64),
}

// Escher doesn't yet support sizing a child slot to its own wrapped content from the parent's
// layout pass — every slot without an explicit `Size` gets an equal share of whatever space is
// left, not a content-fitted one. So instead of one slot per message (which left uneven dead
// space under short turns), the whole transcript is rendered as a single `Body` content block,
// and `Overflow::Scroll` + `ScrollPosition` (see `draw_assistant`) show a window into it —
// following the bottom by default, or pinned wherever the user scrolled to with PageUp/PageDown.
// Was 1 — the "ESCHER TERMINAL ASSISTANT" banner this reserved a row for is gone per the user
// directly ("cool at one point but not useful now"); `0` reclaims that row for the transcript
// instead of leaving a blank gap where the banner used to be.
const HEADER_HEIGHT: u16 = 0;
const FOOTER_HEIGHT: u16 = 3;
const STATUS_HEIGHT: u16 = 1;
/// A blank row between the transcript (or the autocomplete bar, when it's showing) and the
/// input box below it — without this the input's own border sat directly against whatever was
/// above it with no breathing room at all. Keep this at `1`, not `2` — two blank lines reads as
/// more gap than intended; "one space between at most" is the guideline.
const INPUT_GAP_HEIGHT: u16 = 1;

/// Default for `AppState::welcome_tagline` — used verbatim unless `.anvil.toml`'s `[welcome]`
/// table sets `tagline` (see `config.rs`'s `WelcomeConfig`).
const WELCOME_TAGLINE: &str = "A TUI swiss-army-knife for people building Escher apps.";

/// Default for `AppState::welcome_footer` — the same two hint lines `autocomplete_bar_text` always
/// showed, now overridable via `.anvil.toml`'s `[welcome]` table (`footer`, see `config.rs`'s
/// `WelcomeConfig`). One line per `\n`-separated entry.
const DEFAULT_WELCOME_FOOTER: &str = "Esc backs out of whatever you're looking at. Tab expands a tool call's full output.\nType /welcome any time to turn the welcome overview back on.";

/// How long the overlay's position has to sit unchanged before `sync_overlay_bounds_to_
/// persistence` writes it to sqld — long enough that a `Drag` in progress (many events a
/// second while the mouse is moving) never triggers a write per event, short enough that
/// letting go still saves promptly rather than needing a deliberately idle pause afterward.
const OVERLAY_PERSIST_DEBOUNCE: Duration = Duration::from_millis(400);

/// The Relay Console's static file server (see `scripts/serve-relay-console.ts` and
/// `AppState::spawn_relay_console_server`) — one fixed local port, distinct from `shape.rs`'s own
/// `SHAPE_WEB_PORT` (4001) so the two never collide if both happen to be running.
const RELAY_CONSOLE_PORT: u16 = 4002;

/// The real `atlas-relay` server `/relay-console` connects to by default (see
/// `AppState::spawn_relay_server`) — matches `atlas-relay`'s own standalone binary's default
/// (`cargo run -p atlas-relay --bin atlas-relay`), so anyone already running that separately still
/// lands on the same port; also matches the console page's own hardcoded default URL.
const RELAY_PORT: u16 = 9200;

/// How long a copy-selection problem shows a short warning before the status line escalates to
/// the fuller hint about the terminal's mouse-override shortcut.
const MOUSE_HINT_DELAY: Duration = Duration::from_secs(6);
/// Past this age with no further failed attempts, the hint is treated as stale and stops
/// showing even though `mouse_trouble_since` hasn't been explicitly cleared, so a one-off
/// failed copy doesn't leave a permanent warning for a user who's moved on.
const MOUSE_HINT_MAX_AGE: Duration = Duration::from_secs(30);

/// How long the "shell said nah" status hint shows after the shell fallback rejects input
/// outright (see `process::ShellOutcome::Rejected`). Short and non-escalating on purpose — unlike
/// the mouse-trouble hint above, this is expected to fire often during ordinary use (typos, stray
/// text), so it needs to get out of the way quickly rather than accumulate into a second warning.
const SHELL_REJECTED_HINT_DURATION: Duration = Duration::from_secs(3);

/// How long a second Esc at the root (`Page::Chat`, nothing selected) counts as confirming the
/// first one, actually quitting the app — see `AppState::exit_warned_since`.
const EXIT_CONFIRM_WINDOW: Duration = Duration::from_secs(2);

/// Shared token source for this app's terminal UI *and* its native AppKit toolbar/tab strip (see
/// `ThemeState` in `escher_appkit::bevy`, populated from this same instance in `main()`) — one
/// palette read by both surfaces instead of each hardcoding its own. See `escher-styleguide` for
/// the parser and `spec/design/styleguide/anvil.md` for the actual token values.
static STYLEGUIDE: LazyLock<escher_styleguide::Styleguide> =
    LazyLock::new(|| escher_styleguide::Styleguide::parse(include_str!("../../../spec/design/styleguide/anvil.md")).expect("spec/design/styleguide/anvil.md must parse"));

fn styleguide_color(name: &str, fallback: (u8, u8, u8)) -> (u8, u8, u8) {
    // Fully-qualified: `OwoColorize`'s blanket `.color()` (imported for terminal text coloring
    // throughout this file) would otherwise shadow `Styleguide`'s own inherent method of the same
    // name during ordinary `.color()` method-call resolution.
    escher_styleguide::Styleguide::color(&STYLEGUIDE, name).unwrap_or(fallback)
}

fn styleguide_text_size(name: &str, fallback: f64) -> f64 {
    STYLEGUIDE.text_size(name).unwrap_or(fallback)
}

// A small accent palette, reused across borders and message coloring for a cohesive look — each
// value comes from `spec/design/styleguide/anvil.md`, with the original hardcoded value kept as a fallback in
// case that file is ever missing a token (it shouldn't be, in the normal build).
static ACCENT_BLUE: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("accent", (97, 175, 239)));
static ACCENT_ORANGE: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("accent-warn", (204, 154, 77)));
static GREEN: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("success", (106, 153, 85)));
static RED: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("danger", (217, 83, 79)));
static DIM: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("text-muted", (138, 138, 138)));
static BACKGROUND: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("background", (32, 32, 32)));

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const OVERLAY_WIDTH: u16 = 34;

/// A known `/command` — matched and completed against by name. `task`/`scene` are hardcoded
/// (`script: None`, still a `prompt.strip_prefix("/task ")`-style check in the Enter handler
/// below); everything else comes from `discover_js_commands` scanning `commands/*.js`
/// — `script: Some(path)` means running it means spawning `ethos run-command <path> <args>` (see
/// `run_js_command`) rather than any Rust-side logic.
#[derive(Clone)]
struct SlashCommand {
    name: String,
    /// Empty means this command takes no arguments — not just cosmetic (skipped in the
    /// autocomplete bar's `/name <hint>` display), also what decides whether accepting it from
    /// autocomplete submits it directly or completes to `/name ` and waits for args to be typed
    /// (see `draw_assistant`'s `KeyCode::Tab`/`KeyCode::Enter if palette_open` handlers).
    args_hint: String,
    description: String,
    script: Option<PathBuf>,
}

impl SlashCommand {
    /// `args_hint: None` means this command takes no arguments — clearer at the call site than an
    /// empty string, which reads as "forgot to fill this in" rather than "deliberately none."
    /// `script: None` means this command is dispatched by hand in this file's own match, rather
    /// than by running the given script — `discover_js_commands` is the one place today that
    /// always passes `Some(_)`, but a hand-listed command backed by a script is equally legitimate.
    fn new(name: impl Into<String>, args_hint: Option<&str>, description: impl Into<String>, script: Option<PathBuf>) -> Self {
        SlashCommand {
            name: name.into(),
            args_hint: args_hint.unwrap_or_default().to_string(),
            description: description.into(),
            script,
        }
    }
}

fn builtin_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand::new("task", Some("<label>"), "Add a task to the overlay", None),
        SlashCommand::new("relay-console", None, "Open the P2P relay/state debug console", None),
        SlashCommand::new("workspace", None, "List Brainbow projects Ethos's Workspace model discovers under the current directory", None),
        SlashCommand::new("browser", Some("<url>"), "Open a URL in the tabbed browser window", None),
        SlashCommand::new("scene", None, "Open a Bevy window for scene visualization (stub — shows an empty tinted scene)", None),
        SlashCommand::new("shape", None, "Render a fixed demo shape across terminal/web/Unity outputs", None),
        SlashCommand::new("inspect", None, "Inspect live app state (turns, persistence)", None),
        SlashCommand::new("welcome", None, "Toggle the new-user overview shown on an empty transcript", None),
    ]
}

/// Every `.js` file under any `commands/` directory found anywhere below `anvil_root()`
/// (recursively — not just a `commands/` sitting directly under the cwd) becomes a slash
/// command. Missing/unreadable directories just mean no JS commands, not a startup failure —
/// scripts here are optional, not something the app depends on to run.
///
/// The recursive *search for `commands/` itself* (`find_commands_dirs`) matters independently of
/// the recursive walk *within* one once found (`collect_js_scripts`) — confirmed as a real,
/// reported bug: `anvil_root()` is deliberately just the cwd (see its own doc comment), so
/// launching from a real project's own root (this repo's `escher/`, say) rather than from
/// `apps/anvil/` itself — where the actual `commands/` lives, `apps/anvil/commands/` — used to
/// find nothing at all, silently. `/clear`/`/quit` (and anything else under any project's
/// `commands/`) now resolve regardless of which of those two a user happens to be `cd`'d into.
///
/// Falls back to the file's own name for the registered command (so a flat `commands/*.js`
/// project, or `quit.js`/`clear.js` here, works exactly as before), but a script can override its
/// own `command`/`argsHint`/`description` — see `read_exported_string_const` — rather than a
/// nested script always being forced to register under its bare filename regardless of where it
/// lives, or two differently-purposed scripts in different subfolders colliding just because they
/// happen to share a filename. No collision detection if two scripts *do* still declare the same
/// name — last one found (directory-walk order) wins silently; worth a real diagnostic once this
/// sees real multi-author use.
///
/// `NO_ARGS_JS_COMMANDS` (below) is only the *default* for a script that doesn't declare its own
/// `argsHint` — kept for `commands/clear.js`/`commands/quit.js` themselves, which predate this and
/// have no reason to change. `args_hint.is_empty()` (however it's decided) is also what
/// `draw_assistant`'s `KeyCode::Tab`/`KeyCode::Enter if palette_open` handlers check to decide
/// whether accepting the command from the palette submits it directly or completes to `/name `
/// and waits for args to be typed — get a no-args command wrong and it silently needs an extra
/// keypress to actually run.
const NO_ARGS_JS_COMMANDS: &[&str] = &["clear", "quit"];

/// Directory names `find_commands_dirs`/`collect_js_scripts` never descend into — dependency/
/// build output that's both irrelevant (nothing in there is a project's own command script) and
/// expensive to walk (a real `node_modules` or `target` tree is tens of thousands of entries).
/// Anything starting with `.` is skipped too (see `find_commands_dirs`), covering `.git`/
/// `.cargo`/`.output`/etc. without needing to name every one individually.
const SKIP_DIR_NAMES: &[&str] = &["node_modules", "target"];

/// Recursively finds every directory literally named `commands` under `dir`, appending each to
/// `out`. Doesn't descend *into* a `commands/` directory it finds — `collect_js_scripts` handles
/// walking its contents separately — so a `commands/commands/` nested by coincidence would still
/// only ever be found once, as the outer one.
fn find_commands_dirs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.filter_map(|entry| entry.ok()).map(|entry| entry.path()) {
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue };
        if name.starts_with('.') || SKIP_DIR_NAMES.contains(&name) {
            continue;
        }
        if name == "commands" {
            out.push(path);
        } else {
            find_commands_dirs(&path, out);
        }
    }
}

/// A deliberately naive text scan for `export const {name} = "literal";` (or `= null;`) in a
/// command script's source — not a real parse. Discovering the command list by actually running
/// each script through the embedded JS engine (see `process::run_js_command`) just to read one
/// property would mean booting a V8 worker per script before the app even knows what commands
/// exist; a handful of hand-authored, static string-literal declarations don't need that. A
/// script computing its own name/hint dynamically isn't supported — if that turns out to matter,
/// this is the function to replace with a real one, not extend with more string-slicing.
///
/// Returns `None` if `name` isn't exported this way at all (caller should fall back to its own
/// default); `Some(None)` for an explicit `null` (e.g. `argsHint`'s "this command takes no
/// arguments"); `Some(Some(value))` for a quoted string literal.
fn read_exported_string_const(source: &str, name: &str) -> Option<Option<String>> {
    let prefix = format!("export const {name} =");
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix(&prefix) else { continue };
        let rest = rest.trim().trim_end_matches(';').trim();
        if rest == "null" {
            return Some(None);
        }
        for quote in ['"', '\''] {
            if let Some(value) = rest.strip_prefix(quote).and_then(|value| value.strip_suffix(quote)) {
                return Some(Some(value.to_string()));
            }
        }
    }
    None
}

/// Recursively collects every `.js` file under `dir` into `out` — the walk `discover_js_commands`
/// needs, kept separate so that function can stay focused on turning paths into `SlashCommand`s.
/// Skips (rather than fails on) a subdirectory it can't read, same "optional, not load-bearing"
/// treatment `discover_js_commands` already gives a missing top-level `dir`.
fn collect_js_scripts(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.filter_map(|entry| entry.ok()).map(|entry| entry.path()) {
        if path.is_dir() {
            collect_js_scripts(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "js") {
            out.push(path);
        }
    }
}

fn discover_js_commands(root: &Path) -> Vec<SlashCommand> {
    let mut commands_dirs = Vec::new();
    find_commands_dirs(root, &mut commands_dirs);

    let mut paths = Vec::new();
    for commands_dir in &commands_dirs {
        collect_js_scripts(commands_dir, &mut paths);
    }

    let mut commands: Vec<SlashCommand> = paths
        .into_iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).ok()?;
            let file_stem = path.file_stem()?.to_str()?.to_owned();
            // Relative to `root` (the whole search), not just whichever `commands/` this
            // particular script happened to be found under — informative regardless of how
            // deep that `commands/` itself sat.
            let relative_path = path.strip_prefix(root).unwrap_or(path.as_path()).to_string_lossy().into_owned();

            let name = read_exported_string_const(&source, "command").and_then(|declared| declared).unwrap_or(file_stem);

            let description =
                read_exported_string_const(&source, "description").and_then(|declared| declared).unwrap_or_else(|| format!("Run {name} ({relative_path})"));

            let args_hint = match read_exported_string_const(&source, "argsHint") {
                Some(declared) => declared,
                None => (!NO_ARGS_JS_COMMANDS.contains(&name.as_str())).then(|| "<args>".to_string()),
            };

            Some(SlashCommand::new(name, args_hint.as_deref(), description, Some(path)))
        })
        .collect();

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands
}

/// Splits a `/command args...` input into its command name (without the leading `/`) and
/// whatever follows, if it starts with `/` at all. `Some((name, ""))` means the command name
/// itself is still being typed (no space yet) — the autocomplete condition.
fn parse_slash_command(input: &str) -> Option<(&str, &str)> {
    let rest = input.strip_prefix('/')?;
    match rest.split_once(' ') {
        Some((name, args)) => Some((name, args.trim_start())),
        None => Some((rest, "")),
    }
}

/// Every known command matching `partial_name` — `""` matches everything (bare `/` shows the
/// full list), a full command name still matches itself (so Tab/Enter can accept an exact,
/// unambiguous match too, not just narrow a still-ambiguous prefix). Prefix matches come first
/// (typing "s" still lists `/shape`/`/scene` the way it always has), followed by any other command
/// whose name merely *contains* `partial_name` — so a compound name like `relay-console` is still
/// findable by typing "console" alone, not only its literal leading prefix. Without this second
/// pass, typing a remembered fragment of a hyphenated command's name found nothing at all, and
/// Enter would submit that fragment as a literal (nonexistent) command instead. Clones rather
/// than borrows — `commands` is recomputed fresh each frame and these results get captured into
/// a `move` closure below; a handful of small string clones a frame isn't worth threading
/// lifetimes through several closures to avoid.
fn matching_commands(commands: &[SlashCommand], partial_name: &str) -> Vec<SlashCommand> {
    let (prefix, rest): (Vec<_>, Vec<_>) = commands.iter().partition(|command| command.name.starts_with(partial_name));
    prefix.into_iter().chain(rest.into_iter().filter(|command| command.name.contains(partial_name))).cloned().collect()
}

/// Wraps a recognized `/command` prefix in accent color, same `owo_colors` pattern the
/// transcript already uses for diff/role coloring — the terminal surface parses ANSI out of
/// content strings via `ansi_to_tui` regardless of which node they came from, so this needs no
/// support from `Input`/`Scaffold` beyond the display-width fix above.
fn highlight_slash_command(commands: &[SlashCommand], input: &str) -> String {
    let Some((name, _)) = parse_slash_command(input) else {
        return input.to_string();
    };

    let is_recognized = commands.iter().any(|command| command.name == name);
    if !is_recognized {
        return input.to_string();
    }

    // Two distinct colors, not one — the command name (accent blue, bold) marks "command mode";
    // once there's a space and you're typing args, those get their own color (orange, matching
    // the footer's own accent so args reads as "still part of the command," not stray prose) so
    // it's visually clear which part of the input is which as you type. `_owned` bindings have
    // to outlive the `.truecolor()`/`.bold()` calls — those borrow rather than take ownership
    // (a zero-copy styling wrapper, like everywhere else this file uses `owo_colors`) — so they
    // need their own bindings instead of living only as `format!` temporaries.
    let name_owned = format!("/{}", name);
    let highlighted_name = format!("{}", name_owned.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold());

    // The raw suffix exactly as typed — including any trailing whitespace — not
    // `parse_slash_command`'s `rest`, which leading-trims for command-matching purposes. The
    // `Input` element positions its cursor from this string's `display_width` (see
    // `InputValue`'s slot in `packages/core/src/element.rs`), so if this string were ever
    // shorter than the real input (e.g. dropping a trailing space the user actually typed), the
    // cursor would visually lag behind until a non-space character caught it back up.
    let raw_suffix = &input[1 + name.len()..];

    if raw_suffix.is_empty() {
        highlighted_name
    } else {
        let suffix_owned = raw_suffix.to_string();
        let highlighted_suffix = format!("{}", suffix_owned.truecolor(ACCENT_ORANGE.0, ACCENT_ORANGE.1, ACCENT_ORANGE.2));
        format!("{}{}", highlighted_name, highlighted_suffix)
    }
}

/// Rows per mouse wheel tick — smaller than a PageUp/PageDown step, matching how a wheel
/// click is conventionally finer-grained than a page jump.
const MOUSE_SCROLL_STEP: u16 = 3;

/// How far up the transcript the user has scrolled, via PageUp/PageDown or the mouse wheel.
#[derive(Clone, Copy, Debug, Default)]
enum ScrollState {
    /// Always shows the bottom of the transcript, tracking new messages as they arrive.
    #[default]
    Following,
    /// Pinned at a fixed row offset from the top — set by scrolling up, cleared by scrolling
    /// back down to the bottom, or by sending a new message (chat apps always scroll you to
    /// the message you just sent).
    Pinned(u16),
}

/// Scrolls up (back through history) by `step` rows from wherever the transcript currently is
/// — `natural_offset` is where `Following` currently resolves to, since `Pinned` needs a
/// concrete starting point to step back from.
fn scroll_up(scroll: &RwLock<ScrollState>, natural_offset: u16, step: u16) {
    let mut scroll = scroll.write();
    let current = match *scroll {
        ScrollState::Following => natural_offset,
        ScrollState::Pinned(offset) => offset,
    };
    *scroll = ScrollState::Pinned(current.saturating_sub(step));
}

/// Scrolls down (back toward live) by `step` rows; snaps to `Following` once that would reach
/// or pass the bottom, and is a no-op if already `Following` (nothing further down to go).
fn scroll_down(scroll: &RwLock<ScrollState>, natural_offset: u16, step: u16) {
    let mut scroll = scroll.write();
    if let ScrollState::Pinned(offset) = *scroll {
        let next = offset.saturating_add(step);
        *scroll = if next >= natural_offset {
            ScrollState::Following
        } else {
            ScrollState::Pinned(next)
        };
    }
}

/// Which page currently fills the Body area — `Trace` (F2) and `Process` (F3) each independently
/// toggle with `Chat` (see `draw_assistant`'s input handler); everything else (the tasks overlay,
/// the input, `selected_task` browsing) keeps working underneath exactly as before, since this
/// only changes what the Body slot's content resolves to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Chat,
    /// `AppState::trace_buffer`'s raw, unscoped, ANSI-colored `tracing` firehose.
    Trace,
    /// `AppState::process_buffer`'s raw subprocess stdio — see its own doc comment.
    Process,
    /// `/inspect` — live-but-otherwise-invisible app state (turn count, fps, persistence target),
    /// broken into subpages (`InspectSubpage`) cycled with Left/Right. Useful to have somewhere
    /// without permanently occupying the status bar.
    Inspect,
}

/// One screen of `Page::Inspect` — see `inspect_body_text`. `Left`/`Right` cycle through these
/// while `Page::Inspect` is active (see `draw_assistant`'s input handler); `ALL` is the source of
/// truth for that cycle order, so adding a subpage is exactly one line here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectSubpage {
    Session,
    Persistence,
}

impl InspectSubpage {
    const ALL: [InspectSubpage; 2] = [InspectSubpage::Session, InspectSubpage::Persistence];

    fn title(self) -> &'static str {
        match self {
            InspectSubpage::Session => "Session",
            InspectSubpage::Persistence => "Persistence",
        }
    }
}

/// A background command in flight — `AppState::running_command`. Set the moment a command spawns
/// (`spawn_js_command`/`spawn_shell_command`/`spawn_shape_command`), cleared the moment it
/// finishes. Doesn't forcibly switch to `Page::Process` on every command run — jumping to a
/// whole new page per command is overkill. Instead the status line shows a live spinner +
/// elapsed time + a peek at the command's most
/// recent output line (see `draw_assistant`'s `StatusLine` slot) while staying on whatever page
/// was already showing; `F3` is still the "quickly switch to the full stream" shortcut it already
/// was, just no longer forced.
struct RunningCommand {
    label: String,
    started_at: Instant,
}

#[derive(Clone, Resource)]
struct AppState {
    messages: Arc<RwLock<Vec<ChatMessage>>>,
    tasks: Arc<RwLock<Vec<TaskRow>>>,
    user_input: Arc<RwLock<String>>,
    /// The full command list — `task`/`scene` plus whatever `.js` files
    /// `commands/` held at startup. Fixed for the app's lifetime (no live-reload of the commands
    /// directory), so no `RwLock` — nothing ever mutates this after startup. Still `Arc`-wrapped
    /// (not a bare `Vec`) despite that, though: `draw_assistant`'s render closure captures a
    /// clone of this every single frame (it has to — the closure is rebuilt from scratch on every
    /// draw, same as the rest of this app's Scaffold tree), and a bare `Vec<SlashCommand>` clone
    /// would deep-copy every command's `name`/`args_hint`/`description` strings 30 times a
    /// second for data that never changes. `Arc::clone` is just an atomic refcount bump instead —
    /// this follows the same pattern documented in `spec/.agents/principles.md`.
    commands: Arc<Vec<SlashCommand>>,
    /// Which suggestion is highlighted in the slash-command autocomplete list, when it's
    /// showing — navigated with Up/Down, wraps around, reset to 0 whenever the input changes.
    autocomplete_index: Arc<RwLock<usize>>,
    /// Which task's own page is showing in the Body area instead of the transcript, if any —
    /// navigated with Up/Down (when *not* autocompleting — the same keys mean two different
    /// things depending on which overlay is showing) through the tasks list, wrapping through
    /// `None` (the transcript) on both ends rather than clamping, so there's always a way back
    /// to the transcript without a separate keybinding.
    selected_task: Arc<RwLock<Option<usize>>>,
    /// Whether tool calls show their full (fake) output or just a one-line summary. Toggled with Tab.
    expanded: Arc<RwLock<bool>>,
    scroll: Arc<RwLock<ScrollState>>,
    /// Which page fills the Body area — see `Page`'s own doc comment. Toggled with F2.
    page: Arc<RwLock<Page>>,
    /// `Page::Trace`'s own scroll position, entirely separate from `scroll` above — so leaving
    /// the trace page and coming back to `Page::Chat` never disturbs wherever the chat transcript
    /// was scrolled to, and vice versa. Defaults to `Following`, same as `scroll`, so the trace
    /// page opens already tailing the live feed rather than pinned wherever it happened to be
    /// the previous time it was shown.
    trace_scroll: Arc<RwLock<ScrollState>>,
    /// Which `Page::Inspect` subpage is showing — cycled with Left/Right while that page's active
    /// (see `draw_assistant`'s input handler).
    inspect_subpage: Arc<RwLock<InspectSubpage>>,
    /// The Relay Console's static server child process, if this instance has started one and
    /// it's (as of the last check) still alive — see `spawn_relay_console_server`. `Option<
    /// Child>`, not a plain "already started" `AtomicBool` as this used to be: confirmed live as
    /// a real, silent-hang bug otherwise — a `bool` only ever set once true stays true even
    /// after the child it refers to has since died (crashed, or lost a port race against
    /// another instance), so nothing ever notices or retries, and every later `/relay-console`
    /// just waits forever on a `fetch()` to a port nothing is listening on anymore.
    relay_console_server: Arc<Mutex<Option<std::process::Child>>>,
    /// Whether this instance has already started the real `atlas-relay` server the console
    /// connects to (see `spawn_relay_server`) — a plain "spawn once, never again" `AtomicBool`
    /// still works fine here, unlike `relay_console_server` above: `atlas_relay::serve` runs
    /// in-process (a spawned async task, not a child OS process that can die independently of
    /// the code that spawned it), so there's no equivalent "the thing I started earlier quietly
    /// died and I never noticed" failure mode to guard against.
    relay_server_started: Arc<AtomicBool>,
    /// This session's actual `sqld` sync target, resolved once at startup — `Page::Inspect`'s
    /// "Persistence" subpage exists specifically so this (and `session_dir` below) are visible
    /// somewhere, since `--connect`/`ATLAS_SYNC_URL` mean it isn't always the obvious default.
    sqld_target_display: String,
    /// This session's own pid-keyed state directory — see `anvil_session_dir`'s doc comment.
    session_dir: PathBuf,
    /// This instance's own identity — `--identity <name>`, or `anvil-<pid>` if that wasn't given
    /// (see `Args::identity`'s doc comment). Keys `overlay_state` so each person/instance keeps
    /// their own overlay window position instead of sharing one global one (see `ensure_schema`'s
    /// doc comment for the bug this fixes) — everything else in `sqld` (messages, tasks) stays
    /// shared across every instance on purpose; only per-instance UI state like this needs
    /// separating.
    identity: String,
    /// The actual `overlay_state` key derived from `identity` — see `ANVIL_IDENTITY_NAMESPACE`'s
    /// doc comment for why a real, fixed-size UUID is what gets persisted rather than `identity`'s
    /// own arbitrary-length string.
    identity_uuid: uuid::Uuid,
    /// The ring buffer `trace_page_layer` (see `main`) feeds — `Page::Trace`'s content. Already
    /// internally synchronized (`Mutex`), so this is a plain cloneable field rather than another
    /// `Arc<RwLock<_>>` wrapper.
    trace_buffer: LineBuffer,
    /// `Page::Process`'s own scroll position — same independence reasoning as `trace_scroll`.
    process_scroll: Arc<RwLock<ScrollState>>,
    /// Raw subprocess stdio, fed directly by `run_js_command` (not through `tracing` — see
    /// `LineBuffer`'s own doc comment) as each JS command's `ethos` child process runs. Not
    /// scoped to "the currently running command" the way `ChatMessage::Trace`/`live_trace` is —
    /// this is a continuous scrollback across every command run this session, same "everything,
    /// unscoped" shape as `trace_buffer`, just fed from a completely different source (a child
    /// process's literal stdout/stderr bytes, not this process's own `tracing::*` calls).
    process_buffer: LineBuffer,
    /// See `RunningCommand`'s own doc comment.
    running_command: Arc<RwLock<Option<RunningCommand>>>,
    /// Just a clock for animating the fake "running task" spinner in the overlay — not real work.
    start: Instant,
    /// `None` until the background connect task in `AppState::new` finishes, and stays `None`
    /// permanently if `sqld` wasn't reachable within `persistence::CONNECT_TIMEOUT` (see the
    /// `persistence` module). The app runs fully either way, just without persistence, rather
    /// than treating that as fatal or blocking startup on it — a demo shouldn't stall or refuse
    /// to start because a docker-compose service isn't up.
    persistence: Arc<RwLock<Option<Arc<persistence::Persistence>>>>,
    /// The single ordered path every `store.save_*` write goes through — set at the same moment
    /// `persistence` above becomes `Some`, by the same background task (see
    /// `spawn_connect_persistence`). Call sites send a `PersistenceWrite` and return immediately;
    /// one dedicated background task drains this queue one write at a time. Exists so concurrent
    /// writes (several commands/messages fired in quick succession, e.g. from spamming input)
    /// never race each other into the single shared `Connection` — rapid input causing long
    /// stalls raised the suspicion (which couldn't be ruled out without this) that
    /// unsynchronized concurrent access to one `libsql` connection was part of the cause. This
    /// is a "no regret" fix either way: if `libsql`'s `Connection`
    /// already handled concurrent access safely, this just adds strict ordering; if it didn't,
    /// this is the actual correctness fix.
    persistence_writes: Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<PersistenceWrite>>>>,
    /// Bridges the synchronous terminal event loop to `libsql`'s async client — see the
    /// `persistence` module doc comment for why.
    runtime: Arc<tokio::runtime::Runtime>,
    /// Timestamp of every frame drawn in roughly the last second — an actual measured frame
    /// rate, not the theoretical ~30fps ceiling the event loop's poll timeout implies (real
    /// work per frame, e.g. re-wrapping the whole transcript every time, can push it lower).
    frame_times: Arc<RwLock<VecDeque<Instant>>>,
    /// Set the moment a Ctrl+C copy first finds nothing selected, cleared the moment a real
    /// copy succeeds. Drives the mouse-trouble hint in the status line, see `MOUSE_HINT_DELAY`.
    mouse_trouble_since: Arc<RwLock<Option<Instant>>>,
    /// Set the moment the shell fallback rejects typed input outright (see
    /// `process::ShellOutcome::Rejected`), cleared implicitly once `SHELL_REJECTED_HINT_DURATION`
    /// passes — no explicit clear on success, unlike `mouse_trouble_since`, since a *new*
    /// rejection just overwrites this with a fresh timestamp and there's no "fixed" state to
    /// return to in between.
    shell_rejected_since: Arc<RwLock<Option<Instant>>>,
    /// Set the moment Esc is pressed at the root (`Page::Chat`, nothing selected) with no other
    /// way left to back out of anything — a second Esc within `EXIT_CONFIRM_WINDOW` actually
    /// quits (sets `quit_requested`); a first press just warns, so Esc can never quit by
    /// accident. Cleared implicitly once the window passes, same as `shell_rejected_since`.
    exit_warned_since: Arc<RwLock<Option<Instant>>>,
    /// The receiving end of `TranscriptLayer`'s channel (see `main`) — every `tracing::*` call
    /// made while a `live_trace` span is active lands here, drained once per frame in
    /// `draw_assistant` into real `ChatMessage::Trace` entries. A `Receiver` isn't `Sync`, hence
    /// the `Mutex`, even though only `draw_assistant` ever actually locks it.
    trace_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    /// The overlay's last-known persisted position in sqld — `None` until the background
    /// connect+load in `spawn_connect_persistence` finishes (same pop-in-once-loaded behavior
    /// as `messages`/`tasks`), and updated again after every successful debounced save in
    /// `draw_assistant`, so it always reflects what's actually saved rather than only what was
    /// loaded at startup. `draw_assistant` applies it to the surface exactly once, the first
    /// frame it's available *and* the surface has no live override of its own yet — see its own
    /// use site for why that ordering can't clobber a user drag that happened before the load
    /// finished.
    persisted_overlay_bounds: Arc<RwLock<Option<(u16, u16, u16, u16)>>>,
    /// Whether `Page::Chat` shows the new-user overview in place of an empty transcript — a
    /// persisted `/welcome` toggle, defaulting to `true` until loaded (see
    /// `Persistence::load_show_welcome_overview`), so a brand new user sees it on the very first
    /// frame rather than only after the initial sqld round trip finishes.
    show_welcome_overview: Arc<RwLock<bool>>,
    /// `(width, expanded_width)` — the browser's sidebar remembered across launches, same
    /// pop-in-once-loaded contract as `persisted_overlay_bounds`/`show_welcome_overview`.
    /// `spawn_browser_window_on_command` seeds `TabStripState` from this the moment the browser
    /// window is first created; `apply_browser_navigation` writes back to it (via
    /// `PersistenceWrite::SidebarState`) whenever the sidebar's width or collapsed state changes.
    sidebar_state: Arc<RwLock<(f64, f64)>>,
    /// The overlay's position as of the last frame, and when it last changed — drives the
    /// debounce in `draw_assistant` that decides a move/resize gesture has actually settled and
    /// is worth writing to sqld, rather than saving on every single `Drag` event mid-gesture.
    overlay_bounds_seen: Arc<RwLock<Option<Rect>>>,
    overlay_bounds_changed_at: Arc<RwLock<Option<Instant>>>,
    /// `/browser <url>` commands, queued here instead of acted on directly — the `CrosstermEvent`
    /// handler that catches Enter runs inside `TerminalSurface::draw`'s own dispatch, not as a
    /// Bevy system, so it can't take a `MessageWriter<SceneCommand>` directly. Drained by
    /// `AssistantTerminalPlugin::draw_ui` (see `main`) into real `SceneCommand` writes once it's
    /// back in normal system context — the same workaround `escher_bevy::terminal::
    /// TerminalProvider::pending_scenes` already uses for the same reason.
    pending_browser_urls: Arc<Mutex<Vec<String>>>,
    /// A bare `/scene` (no URL) request, queued the same way `pending_browser_urls` is and for the
    /// same reason. Opens a plain Bevy-rendered window for visualizing the app's own running
    /// scene — a stub today (an empty window with a camera), the intended home for a real
    /// scene-inspection view later.
    scene_window_requested: Arc<AtomicBool>,
    /// Set (from `draw_assistant`'s F1 handler, or `--no-tui`'s startup value) to leave the TUI
    /// for the plain raw trace stream `RawStreamGate` prints straight to stdout, cleared (from
    /// `assistant_terminal_draw`'s own raw poll loop, once that mode is active) to return to the
    /// TUI — see `RawStreamGate`'s own doc comment for why this exists as a plain `AtomicBool`
    /// shared with a tracing writer, rather than the `Arc<RwLock<_>>` the rest of this struct
    /// uses for shared state.
    raw_stream: Arc<AtomicBool>,
    /// Set from `spawn_js_command` when a JS command's own output is exactly `QUIT_SENTINEL` (see
    /// `commands/quit.js`) — checked each tick in `assistant_terminal_draw`, same shape as
    /// `raw_stream` above, since a plain `tokio`-spawned future has no `MessageWriter<AppExit>` of
    /// its own to reach for. A script can't exit the process directly (it's a separate `ethos-cli`
    /// child, not this one) — this is the one narrow, explicit bridge that lets it ask to.
    quit_requested: Arc<AtomicBool>,
    /// `false` until `spawn_connect_persistence` knows for certain whether there are real
    /// persisted messages to show — either a successful `load_messages` call actually returned
    /// (empty or not), or the initial `sqld` connect itself failed outright (meaning there's
    /// nothing to load, full stop). `welcome_active` (`draw_assistant`) gates on this — showing
    /// the welcome overview/palette before this is settled, then yanking them away the instant
    /// real persisted messages actually arrive, feels broken. Rendering
    /// nothing during that gap (an empty transcript, since `messages` genuinely is empty until
    /// then) reads as a brief, honest loading state instead of a flash of UI that immediately
    /// contradicts itself.
    startup_settled: Arc<AtomicBool>,
    /// This instance's *starting* choice for whether `/browser`/`/scene` windows float above
    /// every other window — resolved once, in `main`, from `--always-on-top`/`.anvil.toml`. Read
    /// only at window-creation time (`spawn_browser_window_on_command`/`spawn_scene_window_on_
    /// command`); the toolbar's own pin button (`ToolbarEvent::TogglePinned`) changes the *live*
    /// per-window state afterward without touching this.
    always_on_top: bool,
    /// See `config::WelcomeConfig`'s own doc comment — resolved once, in `main`, from
    /// `.anvil.toml`'s `[welcome]` table (or this app's own built-in default text).
    welcome_tagline: String,
    welcome_footer: String,
}

impl AppState {
    /// Starts empty, with no fake seed data, same as any real app — `messages`/`tasks`/
    /// `persistence` populate themselves once the background connect task (spawned below)
    /// finishes, rather than blocking the TUI from appearing at all while that happens. See
    /// `spawn_connect_persistence`.
    async fn new(
        runtime: Arc<tokio::runtime::Runtime>,
        trace_rx: mpsc::Receiver<String>,
        trace_buffer: LineBuffer,
        process_buffer: LineBuffer,
        raw_stream: Arc<AtomicBool>,
        sqld_url: Option<String>,
        identity: String,
        identity_uuid: uuid::Uuid,
        always_on_top: bool,
        welcome_tagline: String,
        welcome_footer: String,
    ) -> Self {
        let mut commands = builtin_commands();
        commands.extend(discover_js_commands(&anvil_root()));
        let commands = Arc::new(commands);

        // Resolved once, up front, rather than re-deriving it wherever `Page::Inspect` needs to
        // show it — `sqld_url` itself is about to move into `spawn_connect_persistence` below.
        let sqld_target_display = sqld_url.clone().unwrap_or_else(|| "http://localhost:8081 (default)".to_string());
        let session_dir = anvil_session_dir();

        let state = AppState {
            messages: Arc::new(RwLock::new(Vec::new())),
            tasks: Arc::new(RwLock::new(Vec::new())),
            user_input: Arc::new(RwLock::new(String::new())),
            commands,
            autocomplete_index: Arc::new(RwLock::new(0)),
            selected_task: Arc::new(RwLock::new(None)),
            expanded: Arc::new(RwLock::new(false)),
            scroll: Arc::new(RwLock::new(ScrollState::default())),
            page: Arc::new(RwLock::new(Page::default())),
            trace_scroll: Arc::new(RwLock::new(ScrollState::default())),
            inspect_subpage: Arc::new(RwLock::new(InspectSubpage::Session)),
            relay_console_server: Arc::new(Mutex::new(None)),
            relay_server_started: Arc::new(AtomicBool::new(false)),
            sqld_target_display,
            session_dir,
            identity,
            identity_uuid,
            trace_buffer,
            process_scroll: Arc::new(RwLock::new(ScrollState::default())),
            process_buffer,
            running_command: Arc::new(RwLock::new(None)),
            start: Instant::now(),
            persistence: Arc::new(RwLock::new(None)),
            persistence_writes: Arc::new(RwLock::new(None)),
            runtime,
            frame_times: Arc::new(RwLock::new(VecDeque::new())),
            mouse_trouble_since: Arc::new(RwLock::new(None)),
            shell_rejected_since: Arc::new(RwLock::new(None)),
            exit_warned_since: Arc::new(RwLock::new(None)),
            trace_rx: Arc::new(Mutex::new(trace_rx)),
            persisted_overlay_bounds: Arc::new(RwLock::new(None)),
            show_welcome_overview: Arc::new(RwLock::new(true)),
            sidebar_state: Arc::new(RwLock::new((220.0, 220.0))),
            overlay_bounds_seen: Arc::new(RwLock::new(None)),
            overlay_bounds_changed_at: Arc::new(RwLock::new(None)),
            pending_browser_urls: Arc::new(Mutex::new(Vec::new())),
            scene_window_requested: Arc::new(AtomicBool::new(false)),
            raw_stream,
            quit_requested: Arc::new(AtomicBool::new(false)),
            startup_settled: Arc::new(AtomicBool::new(false)),
            always_on_top,
            welcome_tagline,
            welcome_footer,
        };

        state.spawn_connect_persistence(sqld_url);
        state
    }

    /// Connects to `sqld` and loads existing messages/tasks on `self.runtime`, without blocking
    /// the caller — the TUI is already interactive by the time this finishes or times out.
    /// `messages`/`tasks` jump from empty to populated once it succeeds, which is a better
    /// experience than staring at a blank terminal for up to `persistence::CONNECT_TIMEOUT`
    /// before anything appears at all.
    fn spawn_connect_persistence(&self, sqld_url: Option<String>) {
        let persistence = self.persistence.clone();
        let persistence_writes = self.persistence_writes.clone();
        let messages = self.messages.clone();
        let tasks = self.tasks.clone();
        let persisted_overlay_bounds = self.persisted_overlay_bounds.clone();
        let show_welcome_overview = self.show_welcome_overview.clone();
        let sidebar_state = self.sidebar_state.clone();
        let startup_settled = self.startup_settled.clone();
        let runtime = self.runtime.clone();
        let identity_uuid = self.identity_uuid;

        self.runtime.spawn(async move {
            let store = match persistence::Persistence::connect(sqld_url.as_deref()).await {
                Ok(store) => store,
                Err(error) => {
                    tracing::warn!("Could not connect to sqld — running without persistence: {error}");
                    // Settled either way — no persistence at all means there's nothing to load,
                    // which is just as final an answer as a successful-but-empty load.
                    startup_settled.store(true, Ordering::Relaxed);
                    return;
                }
            };

            match store.load_messages().await {
                Ok(loaded) => *messages.write() = loaded,
                Err(error) => tracing::warn!("Failed to load messages from sqld, starting empty: {error}"),
            }
            startup_settled.store(true, Ordering::Relaxed);

            match store.load_tasks().await {
                Ok(loaded) => *tasks.write() = loaded,
                Err(error) => tracing::warn!("Failed to load tasks from sqld, starting empty: {error}"),
            }

            match store.load_overlay_bounds(&identity_uuid).await {
                Ok(loaded) => *persisted_overlay_bounds.write() = loaded,
                Err(error) => tracing::warn!("Failed to load overlay position from sqld, using the default: {error}"),
            }

            match store.load_show_welcome_overview(&identity_uuid).await {
                Ok(loaded) => *show_welcome_overview.write() = loaded,
                Err(error) => tracing::warn!("Failed to load the welcome-overview setting from sqld, defaulting to shown: {error}"),
            }

            match store.load_sidebar_state(&identity_uuid).await {
                Ok(loaded) => *sidebar_state.write() = loaded,
                Err(error) => tracing::warn!("Failed to load the sidebar's saved state from sqld, using the default: {error}"),
            }

            let store = Arc::new(store);
            *persistence.write() = Some(store.clone());

            // Without this, a second instance pointed at the same primary (e.g. via `--connect`)
            // never sees writes made by another instance after it started — `connect_inner` only
            // ever syncs once. `2s` is deliberately short: this exists for co-working/testing,
            // where "did my collaborator's message show up yet" is exactly what someone's staring
            // at the screen waiting on, not a background sync people are meant to forget about.
            // Length-comparison before replacing `messages`/`tasks` wholesale, rather than always
            // overwriting: both are strictly append/replace-whole-snapshot by construction (no
            // in-place edits anywhere in this app), so "did the count change" is a cheap, exact
            // stand-in for "did the content change" without diffing.
            const PERSISTENCE_RESYNC_INTERVAL: Duration = Duration::from_secs(2);
            {
                let store = store.clone();
                let messages = messages.clone();
                let tasks = tasks.clone();
                runtime.spawn(async move {
                    loop {
                        tokio::time::sleep(PERSISTENCE_RESYNC_INTERVAL).await;

                        if let Err(error) = with_sqld_timeout(store.sync()).await {
                            tracing::warn!("Periodic sqld resync failed: {error}");
                            continue;
                        }

                        match with_sqld_timeout(store.load_messages()).await {
                            Ok(loaded) if loaded.len() != messages.read().len() => *messages.write() = loaded,
                            Ok(_) => {}
                            Err(error) => tracing::warn!("Failed to reload messages after resync: {error}"),
                        }

                        match with_sqld_timeout(store.load_tasks()).await {
                            Ok(loaded) if loaded.len() != tasks.read().len() => *tasks.write() = loaded,
                            Ok(_) => {}
                            Err(error) => tracing::warn!("Failed to reload tasks after resync: {error}"),
                        }
                    }
                });
            }

            // The single writer — see `AppState::persistence_writes`'s doc comment for why this
            // exists. Every write goes through this one task, one at a time, in the order it was
            // sent; nothing else ever touches `store` directly.
            let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<PersistenceWrite>();
            *persistence_writes.write() = Some(sender);

            runtime.spawn(async move {
                while let Some(first) = receiver.recv().await {
                    // Drains everything already queued behind `first`, not just `first` alone —
                    // this is where "be more selective about what gets written and when" actually
                    // happens. `Message` writes are real user content, so every one still gets
                    // persisted, in order. `Tasks`/`OverlayBounds` are both full-snapshot
                    // replacements of "whatever's true right now" — a burst of either (rapid
                    // status cycling, or dragging/settling the overlay) only needs its *last*
                    // snapshot to ever reach disk, so every earlier one in the same batch is
                    // simply skipped rather than writing (and overwriting) each intermediate one.
                    let mut batch = vec![first];
                    while let Ok(write) = receiver.try_recv() {
                        batch.push(write);
                    }

                    let mut latest_overlay_bounds = None;
                    let mut latest_tasks = None;
                    let mut latest_sidebar_state = None;

                    for write in batch {
                        match write {
                            PersistenceWrite::Message(message) => {
                                if let Err(error) = with_sqld_timeout(store.save_message(&message)).await {
                                    tracing::warn!("Failed to persist message to sqld: {error}");
                                }
                            }
                            PersistenceWrite::Tasks(tasks) => {
                                latest_tasks = Some(tasks);
                            }
                            PersistenceWrite::OverlayBounds { bounds, persisted_overlay_bounds } => {
                                latest_overlay_bounds = Some((bounds, persisted_overlay_bounds));
                            }
                            PersistenceWrite::ShowWelcomeOverview(show) => {
                                if let Err(error) = with_sqld_timeout(store.save_show_welcome_overview(&identity_uuid, show)).await {
                                    tracing::warn!("Failed to persist the welcome-overview setting to sqld: {error}");
                                }
                            }
                            PersistenceWrite::SidebarState(width, expanded_width) => {
                                latest_sidebar_state = Some((width, expanded_width));
                            }
                        }
                    }

                    if let Some((width, expanded_width)) = latest_sidebar_state
                        && let Err(error) = with_sqld_timeout(store.save_sidebar_state(&identity_uuid, width, expanded_width)).await
                    {
                        tracing::warn!("Failed to persist the sidebar's state to sqld: {error}");
                    }

                    if let Some(tasks) = latest_tasks
                        && let Err(error) = with_sqld_timeout(store.save_tasks(&tasks)).await
                    {
                        tracing::warn!("Failed to persist tasks to sqld: {error}");
                    }

                    if let Some((bounds, persisted_overlay_bounds)) = latest_overlay_bounds
                        && let Err(error) = with_sqld_timeout(store.save_overlay_bounds(&identity_uuid, bounds)).await
                    {
                        tracing::warn!("Failed to persist overlay position to sqld: {error}");
                        // Rolled back only on confirmed failure — see the equivalent comment at
                        // the call site (`sync_overlay_bounds_to_persistence`) for why this
                        // doesn't just unconditionally clear it.
                        let mut current = persisted_overlay_bounds.write();
                        if *current == Some(bounds) {
                            *current = None;
                        }
                    }
                }
            });
        });
    }

    /// Runs a JS slash command's script on a background thread and appends the resulting
    /// messages once it finishes, instead of blocking the render thread on `run_js_command` for
    /// however long the child process takes (a fresh build, well over a minute, in the worst
    /// case, see `run_js_command`'s doc comment). The `User` message the caller already pushed
    /// stays visible and the UI stays interactive the whole time. The `Tool`/`Assistant` reply
    /// just appears a moment later, the same way `spawn_connect_persistence` above lets
    /// `messages`/`tasks` populate after the fact instead of blocking startup.
    fn spawn_js_command(&self, command_name: String, script: PathBuf, args: String) {
        let messages = self.messages.clone();
        let persistence = self.persistence.clone();
        let process_buffer = self.process_buffer.clone();
        let running_command = self.running_command.clone();
        let quit_requested = self.quit_requested.clone();

        *running_command.write() = Some(RunningCommand { label: command_name.clone(), started_at: Instant::now() });

        self.runtime.spawn(async move {
            // Named `live_trace`, not the command's own name — `TranscriptLayer` only checks
            // for a span with this exact name anywhere in an event's scope, deliberately not
            // *which* command, since only one command runs at a time in this app today. Entered
            // again inside `spawn_blocking`'s closure below, not just here: span context is
            // thread-local, and that closure runs on a fresh blocking-pool thread that wouldn't
            // otherwise inherit it.
            let span = tracing::info_span!("live_trace", command = %command_name);
            let result = {
                let span = span.clone();
                let command_label = command_name.clone();
                tokio::task::spawn_blocking(move || {
                    let _entered = span.enter();
                    process::run_js_command(&script, &args, &command_label, &process_buffer)
                })
                .await
                .unwrap_or_else(|error| Err(format!("js command task panicked: {error}")))
            };

            if matches!(&result, Ok(output) if output.trim() == QUIT_SENTINEL) {
                quit_requested.store(true, Ordering::Relaxed);
            }

            // `CLEAR_SENTINEL` means `commands/clear.js` actually wiped `sqld` — clear the live
            // transcript too (including the `/clear` invocation itself, already pushed into
            // `messages` by the caller before this task started), so the command visibly does
            // something instead of silently leaving every prior line on screen.
            if matches!(&result, Ok(output) if output.trim() == CLEAR_SENTINEL) {
                messages.write().clear();
                *running_command.write() = None;
                return;
            }

            // An empty success means "ran fine, nothing worth recording" — not "here's your
            // reply text." A real failure still returns actual text (`Err`, or `Ok` with a
            // non-empty message), so those still show up normally.
            if matches!(&result, Ok(output) if output.is_empty()) {
                *running_command.write() = None;
                return;
            }

            let reply = match result {
                Ok(output) => output,
                Err(error) => format!("Script failed: {error}"),
            };

            let new_messages = [
                ChatMessage::Tool { name: "js".into(), detail: command_name, output: vec![] },
                ChatMessage::Assistant(reply),
            ];

            // Cloning the `Arc<Persistence>` out and dropping the guard immediately, rather than
            // holding `persistence.read()`'s guard across the `.await`s below, matters here in a
            // way it doesn't for the synchronous `runtime.block_on` call sites elsewhere in this
            // file. `parking_lot`'s guards are `!Send`, and this whole block is inside a future
            // handed to `runtime.spawn` (a real background task, pollable from any worker
            // thread) rather than `block_on` (always driven from the calling thread). A `!Send`
            // guard held across an await makes the enclosing future itself `!Send`, which
            // `spawn` rejects at compile time.
            let store = persistence.read().clone();
            if let Some(store) = store {
                for message in &new_messages {
                    if let Err(error) = store.save_message(message).await {
                        tracing::warn!("Failed to persist message to sqld: {error}");
                    }
                }
            }

            *running_command.write() = None;
            messages.write().extend(new_messages);
        });
    }

    /// Mirrors `spawn_js_command`, but runs `prompt` against a configured shell instead of an
    /// Ethos script — the input handler's fallback for anything that isn't a recognized slash
    /// command (see the final `else if` branch below). Unlike every other command path in this
    /// app, the caller doesn't record a `User` message before calling this: shell-passthrough
    /// input is frequently a typo or stray text rather than a real command, and the shell itself —
    /// exit 127, "command not found" — is the only thing that can actually tell the two apart. So
    /// this records `User`/`Tool`/`Assistant` together, only once `process::run_shell_command`
    /// confirms the shell didn't reject it outright; a rejection instead just timestamps
    /// `shell_rejected_since` for the status line's brief "shell said nah" and leaves no trace in
    /// the transcript, `process_buffer`, or persistence at all.
    fn spawn_shell_command(&self, prompt: String) {
        let messages = self.messages.clone();
        let persistence = self.persistence.clone();
        let process_buffer = self.process_buffer.clone();
        let running_command = self.running_command.clone();
        let shell_rejected_since = self.shell_rejected_since.clone();
        let user_input = self.user_input.clone();
        let shell = process::resolve_shell_backend();

        *running_command.write() = Some(RunningCommand { label: prompt.clone(), started_at: Instant::now() });

        self.runtime.spawn(async move {
            let span = tracing::info_span!("live_trace", command = %prompt);
            let outcome = {
                let span = span.clone();
                let prompt_label = prompt.clone();
                tokio::task::spawn_blocking(move || {
                    let _entered = span.enter();
                    process::run_shell_command(&shell, &prompt_label, &process_buffer)
                })
                .await
                .unwrap_or_else(|error| process::ShellOutcome::Ran(Err(format!("shell command task panicked: {error}"))))
            };

            *running_command.write() = None;

            let result = match outcome {
                process::ShellOutcome::Rejected => {
                    *shell_rejected_since.write() = Some(Instant::now());

                    // A router-script gut check: Rust has no opinion on what the script does
                    // internally (a local Ollama tool-calling check), nor on what's even
                    // routable — that whole domain, including discovering
                    // which commands exist, belongs to the scripting layer so it stays portable
                    // and hot-swappable without ever touching this file (see `run_reject_router`
                    // and `commands/route.ts`'s own doc comments). Rust hands over the raw
                    // rejected text and nothing else.
                    //
                    // Deliberately does *not* touch `running_command` — a
                    // real local-model round trip measured at 5-20s (mostly one-time
                    // model-load cost) is slow enough that occupying the one shared status-line
                    // slot for its whole duration blocked every other command's own status from
                    // showing (worse: two tasks racing to set/clear the same `Option` could clobber
                    // each other's display outright). This runs fully backgrounded instead — free
                    // to fire off several of these in a row without any of them competing for
                    // screen space with real, foreground commands.
                    let router_result = {
                        let prompt = prompt.clone();
                        tokio::task::spawn_blocking(move || {
                            process::run_reject_router(std::path::Path::new("commands/route.ts"), &prompt)
                        })
                        .await
                        .unwrap_or_else(|error| {
                            tracing::warn!("Reject-router task panicked: {error}");
                            process::RejectRouterResult { replace: None, reply: None }
                        })
                    };

                    // Persisted the same way either kind of note is, once there's actually one to
                    // save — a match found (or a reply given) seconds after the fact still needs
                    // to be visible on scrollback, not depend on a since-vanished status-line hint
                    // or an input box the user may have already typed over.
                    let note = if let Some(replacement) = router_result.replace {
                        // Still only ever *populates the input box* — never executes anything
                        // itself — and only if the user hasn't already started typing something
                        // else in the meantime (checked here, since this runs after a real round
                        // trip). Enter is still the only thing that ever runs a command; this just
                        // saves retyping it when it lands early enough to still apply.
                        let mut input = user_input.write();
                        if input.is_empty() {
                            *input = replacement.clone();
                        }
                        drop(input);
                        Some(ChatMessage::Assistant(format!("(background check) found a possible match for {prompt:?}: `{replacement}`")))
                    } else if let Some(reply) = router_result.reply {
                        // No command matched, but the model still had something worth saying — a
                        // typo correction, an answer to a greeting/question, whatever. Shown as a
                        // real reply, not a "(background check)"-prefixed aside, since this is
                        // meant to read as the assistant actually responding, not a diagnostic.
                        Some(ChatMessage::Assistant(reply))
                    } else {
                        None
                    };

                    if let Some(note) = note {
                        let store = persistence.read().clone();
                        if let Some(store) = store
                            && let Err(error) = store.save_message(&note).await
                        {
                            tracing::warn!("Failed to persist background router-check note to sqld: {error}");
                        }
                        messages.write().push(note);
                    } else {
                        tracing::info!("Reject-router found no match (or failed) for: {prompt}");
                    }

                    return;
                }
                process::ShellOutcome::Ran(result) => result,
            };

            let reply = match result {
                Ok(output) if !output.is_empty() => output,
                Ok(_) => "(command ran, no output)".to_string(),
                Err(error) => format!("Command failed: {error}"),
            };

            let new_messages = [
                ChatMessage::User(prompt.clone()),
                ChatMessage::Tool { name: "shell".into(), detail: prompt, output: vec![] },
                ChatMessage::Assistant(reply),
            ];

            let store = persistence.read().clone();
            if let Some(store) = store {
                for message in &new_messages {
                    if let Err(error) = store.save_message(message).await {
                        tracing::warn!("Failed to persist message to sqld: {error}");
                    }
                }
            }

            messages.write().extend(new_messages);
        });
    }

    /// Opens `command` in a real, standalone terminal window (`escher_os::terminal::
    /// open_running`) instead of running it through the piped-stdout shell passthrough — for
    /// anything (like a Bevy app with its own terminal UI) that needs a genuine TTY of its own to
    /// render correctly, which a piped subprocess can't give it. `working_dir` is this workspace's
    /// own root, the same one `cargo run -p <name>` needs to resolve a package name against.
    fn spawn_open_terminal(&self, command: String, working_dir: PathBuf) {
        let messages = self.messages.clone();

        self.runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || escher_os::terminal::open_running(&command, &working_dir))
                .await
                .unwrap_or_else(|error| Err(escher_os::OsError::Failed(format!("task panicked: {error}"))));

            if let Err(error) = result {
                notify(&messages, format!("Failed to open a terminal: {error}"));
            }
        });
    }

    /// Runs tonight's shared "shape" demo through Ethos's UXML/USS codegen tool, via
    /// `process::run_js_command`'s embedded engine — the exact same call any other JS-backed
    /// command already uses — see `projects/ethos/tools/codegen/uxml/from-description.ts` and
    /// its proposal doc. The one JSON result fans out to all three renderers: a colored block pushed straight
    /// into `process_buffer` (`Page::Process`, the terminal leg), a static HTML page written for
    /// the already-running `escher-web` demo server and opened as a browser tab
    /// (`pending_browser_urls`, same queue `/browser` already drains — the web leg), and
    /// `.uxml`/`.uss` written into Aby's
    /// Unity project (the Unity leg — left for a human to actually view in the Editor, see the
    /// proposal doc for why that part isn't verified here).
    /// Starts the Relay Console's static file server (see `scripts/serve-relay-console.ts`) if
    /// this instance doesn't already have one that's still actually alive, then leaves it
    /// running detached — a plain `deno run --allow-net --allow-read` child, not a Rust/axum
    /// service: lightweight script-driven services in this project use Deno's own `serve`
    /// directly, not axum. Re-checks liveness (`try_wait`) every call rather than trusting a
    /// one-way "already started" flag — see `relay_console_server`'s own doc comment for the
    /// real bug that fixed. A second instance losing the port race to a first instance's own
    /// server is the *expected*, harmless outcome of that (one shared console, whichever
    /// instance got there first serves it) — logged at `info`, matching `spawn_relay_server`'s
    /// identical reasoning for the actual relay right below this.
    fn spawn_relay_console_server(&self) {
        let mut child_slot = self.relay_console_server.lock();

        if let Some(child) = child_slot.as_mut() {
            match child.try_wait() {
                Ok(None) => return, // still running
                Ok(Some(status)) => tracing::info!("Relay Console server exited ({status}) — restarting it"),
                Err(error) => tracing::warn!("Failed to check whether the Relay Console server is still running: {error} — restarting it"),
            }
        }

        let script = anvil_root().join("scripts/serve-relay-console.ts");
        let result = std::process::Command::new("deno")
            .args(["run", "--allow-net", "--allow-read", &script.to_string_lossy(), &RELAY_CONSOLE_PORT.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
                tracing::info!("Relay Console server starting on http://127.0.0.1:{RELAY_CONSOLE_PORT}");
                *child_slot = Some(child);
            }
            Err(error) => {
                tracing::warn!("Failed to start the Relay Console server: {error}");
                *child_slot = None;
            }
        }
    }

    /// Starts a real `atlas_relay::serve` on this app's own runtime the first time it's ever
    /// needed — the console should have something real to connect to by default, not just a
    /// page with an empty "Relay" field the user has to fill in and start a
    /// server for themselves first. In-process (`self.runtime.spawn`, not a subprocess) since
    /// `atlas-relay` is a real async library function, not just a CLI — no separate binary to find/
    /// build/track like `ethos-cli`. Same "spawn once, leave it running detached" contract as
    /// `spawn_relay_console_server`: nothing here ever stops it, since a running relay is harmless
    /// to leave up for the rest of this process's life.
    fn spawn_relay_server(&self) {
        if self.relay_server_started.swap(true, Ordering::SeqCst) {
            return;
        }

        self.runtime.spawn(async move {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], RELAY_PORT));
            if let Err(error) = atlas_relay::serve(addr).await {
                // Since this now starts unconditionally at every launch (not just on a real
                // `/relay-console`), "address already in use" is the *expected* outcome for the
                // second and later of several co-working instances on one machine — one shared
                // relay is the whole point, and whichever instance launched first is already
                // serving it. `info`, not `warn` (this crate's own `--log-level` default warns
                // on anything above `info`, so a genuine startup failure elsewhere still gets
                // seen) — only a real bind failure for a reason *other* than "already bound"
                // would be worth a louder level, and this crate doesn't have a portable way to
                // distinguish that from `atlas_relay::serve`'s own error type today.
                tracing::info!("Relay server on {addr} not started: {error} (already served by another instance, most likely)");
            }
        });

        tracing::info!("Relay server starting on ws://127.0.0.1:{RELAY_PORT}/ws");
    }

    /// Waits for `url` to actually respond (via `scripts/open-page.js`, on this app's own
    /// runtime — never blocks the render thread) before opening a browser tab to it, instead of
    /// pushing straight to `pending_browser_urls` and hoping whatever's supposed to be serving
    /// it is already listening by the time the webview requests it. See that script's own doc
    /// comment for the real bug this fixes. `/relay-console` is the first caller; a future
    /// shortcut command pointed at something else that takes a moment to start up should reach
    /// for this too.
    fn spawn_open_page(&self, url: String, timeout_ms: u64, label: String) {
        let messages = self.messages.clone();
        let pending_browser_urls = self.pending_browser_urls.clone();
        let process_buffer = self.process_buffer.clone();
        let running_command = self.running_command.clone();

        *running_command.write() = Some(RunningCommand { label: label.clone(), started_at: Instant::now() });

        self.runtime.spawn(async move {
            let script = anvil_root().join("scripts/open-page.js");
            let args = serde_json::json!({ "url": url, "timeoutMs": timeout_ms }).to_string();

            let span = tracing::info_span!("live_trace", command = %label);
            let result = {
                let span = span.clone();
                let process_buffer = process_buffer.clone();
                let command_label = label.clone();
                tokio::task::spawn_blocking(move || {
                    let _entered = span.enter();
                    process::run_js_command(&script, &args, &command_label, &process_buffer)
                })
                .await
                .unwrap_or_else(|error| Err(format!("open-page task panicked: {error}")))
            };

            *running_command.write() = None;

            match result {
                Ok(output) if output.is_empty() => pending_browser_urls.lock().push(url),
                Ok(output) => notify(&messages, output),
                Err(error) => notify(&messages, format!("Failed to open {url}: {error}")),
            }
        });
    }

    fn spawn_shape_command(&self) {
        let messages = self.messages.clone();
        let persistence = self.persistence.clone();
        let process_buffer = self.process_buffer.clone();
        let pending_browser_urls = self.pending_browser_urls.clone();
        let running_command = self.running_command.clone();

        *running_command.write() = Some(RunningCommand { label: "shape".to_string(), started_at: Instant::now() });

        self.runtime.spawn(async move {
            let span = tracing::info_span!("live_trace", command = "shape");
            let result = {
                let span = span.clone();
                let process_buffer = process_buffer.clone();
                tokio::task::spawn_blocking(move || {
                    let _entered = span.enter();
                    shape::run_shape_command(&process_buffer)
                })
                .await
                .unwrap_or_else(|error| Err(format!("shape command task panicked: {error}")))
            };

            let reply = match &result {
                Ok(url) => format!(
                    "Shape rendered above — opening a tab to {url}, and wrote Shape.uxml/Shape.uss into Aby's Unity project (Assets/UI/Generated/)."
                ),
                Err(error) => format!("Shape command failed: {error}"),
            };

            if let Ok(url) = &result {
                pending_browser_urls.lock().push(url.clone());
            }

            let new_messages = [
                ChatMessage::Tool { name: "shape".into(), detail: "shape".into(), output: vec![] },
                ChatMessage::Assistant(reply),
            ];

            let store = persistence.read().clone();
            if let Some(store) = store {
                for message in &new_messages {
                    if let Err(error) = store.save_message(message).await {
                        tracing::warn!("Failed to persist message to sqld: {error}");
                    }
                }
            }

            *running_command.write() = None;
            messages.write().extend(new_messages);
        });
    }

    /// Records that a frame was just drawn and returns the number drawn in roughly the last
    /// second — i.e. the current frames-per-second.
    fn record_frame_and_measure_fps(&self) -> usize {
        let mut frame_times = self.frame_times.write();
        let now = Instant::now();
        frame_times.push_back(now);
        while frame_times.front().is_some_and(|&t| now.duration_since(t) > Duration::from_secs(1)) {
            frame_times.pop_front();
        }
        frame_times.len()
    }

    /// Formats a message the way it should appear in the transcript, in color and wrapped to
    /// `width` columns with a hanging indent — every message type opens with a gutter (`> `,
    /// `▸ `/`▾ `, or blank), and a *wrapped continuation* line needs to line up under the text
    /// that follows the gutter, not fall back to column 0 (which is all `Paragraph::wrap()`
    /// does on its own — it doesn't have a hanging-indent concept, so this has to be done
    /// before the text ever gets there).
    fn format_line(message: &ChatMessage, expanded: bool, width: usize) -> String {
        match message {
            ChatMessage::User(text) => {
                format!("{}", wrap_hanging(text, "> ", width).truecolor(GREEN.0, GREEN.1, GREEN.2))
            }
            ChatMessage::Assistant(text) => wrap_hanging(text, "  ", width),
            ChatMessage::Tool { name, detail, output } => {
                let marker = if expanded { "▾ " } else { "▸ " };
                let summary = format!(
                    "{}",
                    wrap_hanging(&format!("{}({})", name, detail), marker, width).truecolor(DIM.0, DIM.1, DIM.2)
                );

                if !expanded || output.is_empty() {
                    return summary;
                }

                // The `⎿` marker only ever appears once, on the very first output line — every
                // other line (whether it's a wrapped continuation of that line or a later
                // output entry) hangs under it at the same 4-column indent.
                const OUTPUT_GUTTER: &str = "    ";
                let output_width = width.saturating_sub(OUTPUT_GUTTER.len());

                let mut rendered = vec![summary];
                let mut first_output_line = true;

                for line in output {
                    let (color, text) = Self::diff_color(line);

                    for wrapped in wrap_words(&text, output_width) {
                        let prefix = if first_output_line { "  ⎿ " } else { OUTPUT_GUTTER };
                        first_output_line = false;
                        rendered.push(format!("{}{}", prefix, wrapped.truecolor(color.0, color.1, color.2)));
                    }
                }

                rendered.join("\n")
            }
            ChatMessage::Trace(text) => {
                format!("{}", wrap_hanging(text, "  ", width).truecolor(DIM.0, DIM.1, DIM.2))
            }
        }
    }

    /// A `-`/`+` prefixed line reads as a diff removal/addition; anything else is dim output
    /// text. Determined once per *logical* output line (not per wrapped physical line) so a
    /// wrapped continuation keeps the same color as the line it continues.
    fn diff_color(line: &str) -> ((u8, u8, u8), String) {
        if let Some(rest) = line.strip_prefix('-') {
            (*RED, format!("-{}", rest))
        } else if let Some(rest) = line.strip_prefix('+') {
            (*GREEN, format!("+{}", rest))
        } else {
            (*DIM, line.to_string())
        }
    }

    /// Shown in place of the transcript on `Page::Chat` while it's empty and `show_welcome_
    /// overview` is on (see `draw_assistant`'s `body_content`/`welcome_active`) — just a title, one
    /// short tagline (`WELCOME_TAGLINE`), and the "what it can do" bullets. No longer draws the
    /// command list itself, and no longer explains the mechanics of typing `/` or a bare shell
    /// command: both used to live here, but once the live `AutocompleteBar`
    /// started defaulting open right below this text (see `welcome_active`), a second copy of the
    /// same list read as two competing displays, and the old intro paragraph/closing usage lines
    /// read as redundant after the first few seconds. `WELCOME_TAGLINE` and this whole method's
    /// content are both real candidates to become configurable/editable rather than hardcoded —
    /// see `ROADMAP.md` for that, not built speculatively here.
    fn welcome_overview_text(&self) -> String {
        lines![
            "Welcome to Anvil".truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold(),
            "",
            self.welcome_tagline.as_str(),
            "",
            "What it can do:".truecolor(DIM.0, DIM.1, DIM.2),
            "  - Run ordinary shell commands directly, no prefix needed.",
            "  - Run first-party and project-local slash commands (JS/TS scripts under commands/).",
            "  - Track ad hoc tasks in a floating overlay while you work.",
            "  - Peek at a running background command (F3) or the raw tracing firehose (F2).",
            "  - Open real browser/Bevy-scene windows alongside the terminal (/browser, /scene).",
            "  - Spawn cargo run in its own terminal window, so long-running/interactive programs get a real TTY.",
        ]
        .join("\n")
    }

    /// Content for the floating "running tasks" overlay — real tasks only (`/task <label>` in
    /// the input; status moves via `cycle_selected_task_status`, bound to Left/Right on a
    /// selected task, not automatically — nothing here infers "running"/"done" from any actual
    /// process), the spinner glyph is the only genuinely animated part, from
    /// `self.start.elapsed()`. `selected` (Up/Down navigate it — see `draw_assistant`) picks out
    /// a row with `▸`/bold, same convention as the autocomplete overlay's own selection marker,
    /// and is also which task's own page currently replaces the transcript in the Body area.
    fn tasks_overlay_text(&self, selected: Option<usize>) -> String {
        let spinner = SPINNER_FRAMES[(self.start.elapsed().as_millis() / 80) as usize % SPINNER_FRAMES.len()];
        // A cycling "." → ".." → "..." → "" — the same quiet "still working" cue Claude Code
        // uses on its own status line, distinct from the spinner glyph itself.
        let dots = ".".repeat((self.start.elapsed().as_millis() / 400 % 4) as usize);

        let mut lines = vec![format!("{}", "Tasks".truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold())];

        for (i, task) in self.tasks.read().iter().enumerate() {
            let entry = match task.status.as_str() {
                "done" => format!("{} {}", "✓".truecolor(GREEN.0, GREEN.1, GREEN.2), task.label),
                "running" => format!(
                    "{} {}{}",
                    spinner.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2),
                    task.label,
                    dots.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2),
                ),
                _ /* "pending" or anything unrecognized */ => {
                    format!("{}", format!("○ {}", task.label).truecolor(DIM.0, DIM.1, DIM.2))
                }
            };

            // Just the marker changes on selection, not `entry`'s own status-glyph coloring —
            // recoloring the whole row would clobber "done"/"running"/"pending" info that's
            // useful to still see at a glance even on the selected row.
            let marker = if Some(i) == selected {
                format!("{}", "▸".truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2))
            } else {
                " ".to_string()
            };
            lines.push(format!("{} {}", marker, entry));
        }

        // Doubles as the answer to "how do I add a task"/"how do I look at a task" — nowhere
        // else in the UI says either. This overlay only ever renders with at least one task
        // present (see its call site), so the select/status hint always applies here.
        lines.push(format!("{}", "/task <label>".truecolor(DIM.0, DIM.1, DIM.2)));
        lines.push(format!("{}", "↑↓ select · ←/→ status".truecolor(DIM.0, DIM.1, DIM.2)));

        // Breathing room comes from the overlay's own `Padding::left(1)`/`Padding::right(1)`
        // style now, not a hand-rolled leading blank line + per-line indent — the overlay-height
        // calculation at this function's call site has to stay in sync with the line count here:
        // 1 title + one per task + 2 or 3 hint lines.
        lines.join("\n")
    }
}

/// Renders `commands` as aligned `/name  <args>  description` rows — the one place either kind of
/// command listing actually gets drawn, so the always-visible full list under the welcome message
/// (`AppState::welcome_overview_text`, `selected: None`) and the live-filtered bar that pops up
/// above the input while typing (`autocomplete_bar_text`, `selected: Some(_)`) can never drift out
/// of sync with each other — a tweak to spacing or coloring here changes both at once. Three
/// separately-aligned columns (name, args, description) rather than one combined `/name <args>`
/// column, so args stay easy to spot at a glance even in a longer list; kept to one row per
/// command rather than a row each for name/description, since doubling every row's height is
/// worse for scanability than widening two columns a little, and "same row" is the simplest
/// possible answer to "which description belongs to which command." `selected` picks out one row
/// with a leading `▸` and a bolded name, same convention `tasks_overlay_text`'s own marker uses —
/// only the marker/bold changes, not the row's other colors, so a selected row still shows exactly
/// the same information as any other, just picked out.
fn command_rows_text(commands: &[SlashCommand], selected: Option<usize>) -> Vec<String> {
    let name_labels: Vec<String> = commands.iter().map(|command| format!("/{}", command.name)).collect();
    let name_width = name_labels.iter().map(|label| UnicodeWidthStr::width(label.as_str())).max().unwrap_or(0);
    let args_width = commands.iter().map(|command| UnicodeWidthStr::width(command.args_hint.as_str())).max().unwrap_or(0);

    commands
        .iter()
        .zip(name_labels.iter())
        .enumerate()
        .map(|(index, (command, name_label))| {
            let name_padding = " ".repeat(name_width - UnicodeWidthStr::width(name_label.as_str()) + 2);
            let args_padding = " ".repeat(args_width - UnicodeWidthStr::width(command.args_hint.as_str()) + 2);
            let is_selected = Some(index) == selected;

            let marker = if is_selected { "▸ ".truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).to_string() } else { "  ".to_string() };
            let name_display = if is_selected {
                name_label.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold().to_string()
            } else {
                name_label.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).to_string()
            };

            format!(
                "{marker}{name_display}{name_padding}{}{args_padding}{}",
                command.args_hint.truecolor(ACCENT_ORANGE.0, ACCENT_ORANGE.1, ACCENT_ORANGE.2),
                command.description.as_str().truecolor(DIM.0, DIM.1, DIM.2)
            )
        })
        .collect()
}

/// Content for the dedicated `AutocompleteBar` slot — the one and only place a command listing
/// ever renders (see `AppState::welcome_overview_text`'s own doc comment for why it no longer
/// draws its own copy). Shown two ways: while a `/command` name is being typed, one row per
/// *match* with the current one picked out (Up/Down moves `selected_index`, `Some(_)` here); or,
/// with nothing typed yet, defaulted open showing *every* command with nothing picked out
/// (`None`) for as long as the welcome overview is still active (see `draw_assistant`'s
/// `welcome_active`/`palette_open`) — the thing that lets the welcome message and this bar open
/// together, stay together while typing, and close together on the first real submission, rather
/// than being two independently-triggered displays that just happen to overlap. Plus a keybinding
/// hint (only once there's a real selection to navigate) and a small usage note. No title line or
/// border chrome the way the tasks overlay's content needs — this is an inline bar sitting right
/// above the input, not a standalone floating box, so it doesn't need to caption or frame itself
/// the same way. Line count has to stay in sync with `draw_assistant`'s `autocomplete_bar_height`
/// calculation: one row per command shown + 2 or 3 trailing hint lines.
fn autocomplete_bar_text(matches: &[SlashCommand], selected_index: Option<usize>, footer: &str) -> String {
    let mut lines = command_rows_text(matches, selected_index);

    // A blank line before the trailing hints — without it, the last command row and the
    // "↑↓ navigate" line below it ran together as if they were part of the same list.
    lines.push(String::new());

    if selected_index.is_some() {
        lines.push(format!("{}", "↑↓ navigate · Tab/Enter accept".truecolor(DIM.0, DIM.1, DIM.2)));
    }
    // The same small note `AppState::welcome_overview_text` shows under its own copy of this
    // list for a brand new user — repeated here so a *returning* user (who never sees that
    // one-time welcome page again once `submit_command` auto-dismisses it) still runs into these
    // tips the first time they reach for `/`, not only once, ever, right at the start.
    for line in footer.lines() {
        lines.push(format!("{}", line.truecolor(DIM.0, DIM.1, DIM.2)));
    }

    lines.join("\n")
}

/// The full transcript (every message, not just what currently fits), one blank line between
/// turns. `draw_assistant` shows a scrollable window into this rather than trimming history —
/// trimming is what made old messages disappear for good instead of just scrolling out of view.
fn build_transcript(messages: &[ChatMessage], expanded: bool, width: usize) -> String {
    messages
        .iter()
        .map(|message| AppState::format_line(message, expanded, width))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What the Body area shows in place of the transcript when a task is selected (Up/Down while
/// the tasks overlay is showing, not autocompleting — see `draw_assistant`). Genuinely minimal
/// on purpose: `TaskRow` only carries a label and a status, nothing this app tracks links a task
/// back to specific messages/tool-calls yet, so there's no real detail to show beyond that —
/// better to say so plainly than to fake something.
fn task_detail_text(task: &TaskRow) -> String {
    let status_label = match task.status.as_str() {
        "done" => format!("{}", "Done".truecolor(GREEN.0, GREEN.1, GREEN.2).bold()),
        "running" => format!("{}", "Running".truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold()),
        _ => format!("{}", "Pending".truecolor(DIM.0, DIM.1, DIM.2).bold()),
    };

    format!(
        "{}\n\n{} {}\n\n{}\n\n{}",
        task.label.as_str().truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold(),
        format!("{}", "Status:".truecolor(DIM.0, DIM.1, DIM.2)),
        status_label,
        "No linked activity yet — nothing in this app connects a task back to specific messages \
         or tool calls yet.".truecolor(DIM.0, DIM.1, DIM.2),
        "←/→ change status · Esc back".truecolor(DIM.0, DIM.1, DIM.2),
    )
}

/// Cycles the selected task's status through pending → running → done (`forward`) or the reverse
/// (`Left`), wrapping at both ends, and persists the change — see `PersistenceWrite::Tasks`'s doc
/// comment for why this always sends the *whole* task list rather than just the row that changed.
/// A no-op if nothing's selected or the selection is somehow stale (task list changed underneath
/// it) — nothing to cycle.
fn cycle_selected_task_status(
    tasks: &Arc<RwLock<Vec<TaskRow>>>,
    selected_task: &Arc<RwLock<Option<usize>>>,
    persistence_writes: &Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<PersistenceWrite>>>>,
    forward: bool,
) {
    const CYCLE: [&str; 3] = ["pending", "running", "done"];

    let Some(index) = *selected_task.read() else { return };
    let mut tasks = tasks.write();
    let Some(task) = tasks.get_mut(index) else { return };

    let current = CYCLE.iter().position(|status| *status == task.status).unwrap_or(0);
    let next = if forward { (current + 1) % CYCLE.len() } else { (current + CYCLE.len() - 1) % CYCLE.len() };
    task.status = CYCLE[next].to_string();

    // Same "never block the render thread, a failed save is just logged" tradeoff as every other
    // persistence call site — see `AppState::persistence_writes`'s doc comment.
    if let Some(sender) = persistence_writes.read().clone() {
        let _ = sender.send(PersistenceWrite::Tasks(tasks.clone()));
    }
}

/// Writes the overlay's position to sqld once it's settled — `OVERLAY_PERSIST_DEBOUNCE` after
/// the last actual change, not on every `Drag` event while a gesture is still in progress. Cheap
/// to call every frame regardless: the two locks below are the only cost once the position has
/// stopped changing and is already known to match what's saved.
fn sync_overlay_bounds_to_persistence(surface: &TerminalSurface<CrosstermBackend<Stdout>>, state: &AppState) {
    let current = surface.overlay_bounds();

    {
        let mut seen = state.overlay_bounds_seen.write();
        if *seen != current {
            *seen = current;
            *state.overlay_bounds_changed_at.write() = Some(Instant::now());
        }
    }

    let Some(current) = current else { return };
    let bounds = (current.x, current.y, current.width, current.height);

    if *state.persisted_overlay_bounds.read() == Some(bounds) {
        return;
    }

    let settled = state.overlay_bounds_changed_at.read().is_some_and(|at| at.elapsed() >= OVERLAY_PERSIST_DEBOUNCE);

    if !settled {
        return;
    }

    let Some(sender) = state.persistence_writes.read().clone() else { return };

    // Marked *before* the save actually completes, not after — this function runs every frame
    // from the render loop, so without this, every frame between "just settled" and "the writer
    // task actually getting to it" would see the same stale `persisted_overlay_bounds` and queue
    // its own redundant write. A save that ends up failing just means the next real bounds change
    // tries again — the existing behavior already had no retry beyond that anyway. The writer
    // task (`spawn_connect_persistence`) also coalesces any burst of these down to just the
    // latest one, so this and that are two independent, complementary layers of the same "don't
    // write more than necessary" idea.
    *state.persisted_overlay_bounds.write() = Some(bounds);

    let _ = sender.send(PersistenceWrite::OverlayBounds { bounds, persisted_overlay_bounds: state.persisted_overlay_bounds.clone() });
}

/// `Page::Inspect`'s body — whichever `InspectSubpage` is showing, one screen of live-but-
/// normally-invisible app state ("turns," exactly where persistence points). Useful to have
/// somewhere without permanently occupying the status bar (see `draw_assistant`'s `StatusLine`
/// slot, which dropped the turn count for exactly this reason).
fn inspect_body_text(state: &AppState, subpage: InspectSubpage, fps: usize) -> String {
    let mut tabs = String::new();
    for (i, candidate) in InspectSubpage::ALL.iter().enumerate() {
        if i > 0 {
            tabs.push_str("   ");
        }
        let title = candidate.title();
        if *candidate == subpage {
            let _ = write!(&mut tabs, "{}", format!("[{title}]").truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold());
        } else {
            let _ = write!(&mut tabs, "{}", title.truecolor(DIM.0, DIM.1, DIM.2));
        }
    }

    let body = match subpage {
        InspectSubpage::Session => format!(
            "turns: {}\nfps: {fps}\nuptime: {:.0}s",
            state.messages.read().len(),
            state.start.elapsed().as_secs_f64(),
        ),
        InspectSubpage::Persistence => format!(
            "identity: {} ({})\nsqld target: {}\nconnected: {}\nsession dir: {}",
            state.identity,
            state.identity_uuid,
            state.sqld_target_display,
            state.persistence.read().is_some(),
            state.session_dir.display(),
        ),
    };

    format!("{tabs}\n\n{body}\n\n{}", "←/→ switch subpage".truecolor(DIM.0, DIM.1, DIM.2))
}

fn draw_assistant(
    surface: &mut TerminalSurface<CrosstermBackend<Stdout>>,
    state: &AppState,
) -> Result<TerminalAction> {
    // The terminal size is already known before the scaffold tree is built (unlike the Body
    // slot's own rect, which only exists after layout), so the transcript's total wrapped
    // height — needed to know how far it *can* scroll — has to be computed out here rather
    // than inside the `slot::<Body>` closure below.
    let area = surface.size()?;
    let expanded = *state.expanded.read();
    let fps = state.record_frame_and_measure_fps();

    // Drains whatever `TranscriptLayer` forwarded since the last frame (see `main`) — a
    // currently-running command's own `tracing::*` calls and `console.log` output, appended
    // live rather than waiting for the whole command to finish. `try_recv` never blocks, so an
    // idle channel costs nothing here.
    {
        let trace_rx = state.trace_rx.lock();
        let mut messages = state.messages.write();
        while let Ok(line) = trace_rx.try_recv() {
            messages.push(ChatMessage::Trace(line));
        }
    }

    // Applies whatever `spawn_connect_persistence`'s background load found in sqld, the first
    // frame it's available — `surface.overlay_bounds().is_none()` guards this to exactly once:
    // once applied (from here or from a real drag), it's never `None` again for the rest of the
    // session, so this can never fire twice or clobber a user drag that happened to land before
    // the load finished (in that case the surface already has `Some` by the time this runs, so
    // the stale persisted value is correctly skipped instead of overwriting it).
    if surface.overlay_bounds().is_none()
        && let Some((x, y, width, height)) = *state.persisted_overlay_bounds.read()
    {
        surface.set_overlay_bounds(Some(Rect { x, y, width, height }));
    }

    sync_overlay_bounds_to_persistence(surface, state);

    // Autocomplete is active exactly while the command *name* is still being typed — a `/`
    // with no space after it yet — and at least one known command starts with it. Checked
    // directly rather than through `parse_slash_command` (whose `(name, "")` shape can't
    // distinguish "no space typed yet" from "space typed, args still empty" — the latter means
    // the command's already been chosen, e.g. right after Tab-accepting one, and shouldn't
    // reopen the dropdown). Computed once here, outside the input closure below, since the
    // `AutocompleteBar` slot's content/height and the Input element's displayed value all need
    // it, and `body_height` (right below) needs to account for the bar's height too.
    let autocomplete_matches: Vec<SlashCommand> = {
        let input = state.user_input.read();
        match input.strip_prefix('/') {
            Some(name) if !name.contains(' ') => matching_commands(&state.commands, name),
            _ => Vec::new(),
        }
    };
    let is_autocompleting = !autocomplete_matches.is_empty();

    // Whether the welcome message is (still) the thing occupying the Body area — see the
    // `Page::Chat` arm of `body_content`'s own `match`, below, which this has to agree with
    // exactly. Defaults the palette open below on the strength of this alone, deliberately not
    // `is_autocompleting` — that's what makes the welcome message and the palette open together,
    // stay open together while typing, and close together the moment `submit_command` dismisses
    // the welcome overview, rather than two independently-triggered displays that just happen to
    // overlap.
    let welcome_active =
        state.startup_settled.load(Ordering::Relaxed) && state.messages.read().is_empty() && *state.show_welcome_overview.read();

    // Real autocomplete (a `/name` actually being typed) always wins and shows filtered matches;
    // failing that, defaulting open onto every command for as long as `welcome_active` holds is
    // what lets a first-time user see (and actually navigate/pick from)
    // the palette before they've typed anything at all — closing it the instant either stops
    // being true. Cloned (small, cheap lists) rather than borrowed — `autocomplete_matches` itself
    // gets moved into the `draw_with_poll_timeout` closure further down, which a borrow of it
    // couldn't outlive.
    let palette_matches: Vec<SlashCommand> = if is_autocompleting {
        autocomplete_matches.clone()
    } else if welcome_active {
        state.commands.as_ref().clone()
    } else {
        Vec::new()
    };
    let palette_open = !palette_matches.is_empty();
    // Deliberately narrower than `palette_open`: real filtering (`is_autocompleting`), or the
    // idle-open case *with the input still genuinely empty* — not just "the palette happens to be
    // showing." Without the emptiness check, typing ordinary free text while the feed's still
    // empty (`welcome_active` alone) kept `palette_open` true the whole time, and Up/Down/Tab/
    // Enter would hijack every keystroke into palette navigation/acceptance instead of composing
    // and submitting that text — a real bug, not a hypothetical. Once there's
    // real (non-`/`) text in the input, the palette stays visible underneath as calm reference
    // (see `body_content`'s `welcome_active` arm) but stops being the thing Up/Down/Tab/Enter act
    // on.
    let palette_interactive = is_autocompleting || (welcome_active && state.user_input.read().is_empty());
    // Valid — and navigable/acceptable via Up/Down/Tab/Enter, see the keyboard handlers below —
    // any time the palette is actually interactive: with nothing typed yet this defaults to row
    // 0, the same "nothing chosen yet, first row picked out" starting point `tasks_overlay_text`'s
    // own `selected` already uses.
    let palette_selected_index = if palette_interactive { (*state.autocomplete_index.read()).min(palette_matches.len() - 1) } else { 0 };

    // 0 rows (hidden) when the palette isn't open at all; otherwise one row per command shown + 1
    // blank separator + 1 nav-key hint + one row per `state.welcome_footer` line (configurable,
    // see `config.rs`'s `WelcomeConfig` — same count `autocomplete_bar_text` actually renders) + 1
    // for the slot's own `Padding::top`, so that breathing room never eats into a real content row.
    let autocomplete_bar_height = if palette_open { palette_matches.len() as u16 + 3 + state.welcome_footer.lines().count() as u16 } else { 0 };

    let body_height = area.height.saturating_sub(HEADER_HEIGHT + autocomplete_bar_height + INPUT_GAP_HEIGHT + FOOTER_HEIGHT + STATUS_HEIGHT);
    // Body gets a 1-column pad on each side (below) so its text lines up with the Footer's
    // input, which is 1 column in from its own border — so text has to wrap 2 columns
    // narrower than the raw terminal width to end up in the same place at render time.
    let body_width = area.width.saturating_sub(2) as usize;

    let input_display = highlight_slash_command(&state.commands, &state.user_input.read());

    // Only rendered once there's at least one task (see this closure's tail below) — command
    // suggestions moved to their own `AutocompleteBar` slot above the input (see
    // `autocomplete_bar_height`/`autocomplete_bar_text`), so this slot no longer needs to swap
    // content with autocomplete (`Scaffold` only supports one detached overlay at a time — see
    // `overlay`'s doc comment in `escher_core::scaffold` — but there's nothing left to share it
    // with here). Interior rows: 1 title + one row per task + 1 hint line ("/task <label>") + 1
    // for the always-applicable "↑↓ select · ←/→ status" line. Plus 2 for the border — no extra
    // rows for padding, since the overlay's `Padding::left(1)`/`Padding::right(1)` is
    // horizontal-only.
    let task_count = state.tasks.read().len() as u16;
    let overlay_height = task_count + 5;

    // Up/Down (when not autocompleting) selects a task from the overlay instead of scrolling —
    // when one's selected, the Body area shows that task's own page instead of the transcript.
    let selected_task_index = *state.selected_task.read();
    let selected_task = selected_task_index.and_then(|index| state.tasks.read().get(index).cloned());

    // F2/F3 (see the input handler below) swap the whole Body area to the raw tracing firehose
    // (`Page::Trace`) or raw subprocess stdio (`Page::Process`) instead of the transcript/task-
    // detail page — takes priority over `selected_task` since these are independent axes (a
    // task can stay selected underneath while briefly checking another page, the same way it
    // stays selected across any other frame that doesn't touch it).
    let page = *state.page.read();
    let body_content = match page {
        Page::Trace => state.trace_buffer.snapshot(),
        Page::Process => state.process_buffer.snapshot(),
        Page::Inspect => inspect_body_text(state, *state.inspect_subpage.read(), fps),
        Page::Chat => match &selected_task {
            Some(task) => task_detail_text(task),
            None => {
                let messages = state.messages.read();
                // Stays showing while the autocomplete bar is up, rather than hiding — the full
                // `welcome_active` (computed once, above, alongside the `AutocompleteBar`'s own
                // `palette_open`) rather than re-deriving the same condition here — the whole
                // point is that this text and the palette open and close in lockstep, so there's
                // exactly one place deciding "is the welcome message still active" for both.
                if welcome_active {
                    state.welcome_overview_text()
                } else {
                    build_transcript(&messages, expanded, body_width)
                }
            }
        },
    };

    let mut height_counter = LineCounter::<u16>::new(body_width);
    let _ = write!(&mut height_counter, "{}", body_content);
    let content_height = height_counter.count();
    let natural_offset = content_height.saturating_sub(body_height);

    // `Page::Trace`/`Page::Process` each have their own, entirely separate `ScrollState`
    // (`trace_scroll`/`process_scroll`) — so toggling away to `Page::Chat` and back never
    // disturbs wherever the chat transcript (or the other page) was scrolled to, and vice versa
    // (see `AppState::trace_scroll`'s doc comment).
    let active_scroll = match page {
        Page::Trace => &state.trace_scroll,
        Page::Process => &state.process_scroll,
        Page::Inspect | Page::Chat => &state.scroll,
    };
    let scroll_offset = match *active_scroll.read() {
        ScrollState::Following => natural_offset,
        ScrollState::Pinned(offset) => offset.min(natural_offset),
    };
    let is_scrolled_up = scroll_offset < natural_offset;

    // A scroll offset can only skip rows from the top — it can't manufacture rows that don't
    // exist. So when the content already fits in the viewport (nothing to scroll to in the
    // first place), pad it with leading blank rows instead, so it still sits at the bottom the
    // way a chat app (or a `tail -f`) does, rather than pinned to the top. Only for the
    // transcript and the two raw-feed pages, though — a task's own page reads top-down like a
    // document, not a growing log, so it stays pinned to the top instead.
    let pads_to_bottom = match page {
        Page::Trace | Page::Process => true,
        Page::Inspect => false,
        Page::Chat => selected_task.is_none(),
    };
    let body_content = if pads_to_bottom && natural_offset == 0 {
        let padding_rows = body_height.saturating_sub(content_height);
        "\n".repeat(padding_rows as usize) + body_content.as_str()
    } else {
        body_content
    };

    // A zero poll timeout, not the library's own default ~33ms — this runs as a Bevy `PreUpdate`
    // system, one call per tick, and Bevy's own reactive scheduling (window/device events,
    // `spawn_input_watcher`) already decides when a tick is worth running at all. Blocking here
    // too, on top of that, stalls Bevy's *entire* main thread — rendering, animation, everything
    // — for up to the full timeout on every tick that doesn't happen to have an event already
    // waiting. This was capping the whole app's effective frame rate to a fraction of what Bevy
    // itself could otherwise sustain, worse since the input-lag fix above made this call happen
    // up to twice per tick. See `TerminalSurface::draw_with_poll_timeout`'s own doc comment.
    let action = surface.draw_with_poll_timeout(move |terminal_root| {
        let root = terminal_root
            .handle::<CrosstermEvent>({
                let user_input = state.user_input.clone();
                let autocomplete_index = state.autocomplete_index.clone();
                let messages = state.messages.clone();
                let tasks = state.tasks.clone();
                let expanded_flag = state.expanded.clone();
                let scroll = state.scroll.clone();
                let trace_scroll = state.trace_scroll.clone();
                let process_scroll = state.process_scroll.clone();
                let page = state.page.clone();
                let inspect_subpage = state.inspect_subpage.clone();
                let persistence_writes = state.persistence_writes.clone();
                let commands = state.commands.clone();
                let selected_task = state.selected_task.clone();
                let pending_browser_urls = state.pending_browser_urls.clone();
                let scene_window_requested = state.scene_window_requested.clone();
                let exit_warned_since = state.exit_warned_since.clone();
                let show_welcome_overview = state.show_welcome_overview.clone();
                let quit_requested = state.quit_requested.clone();
                let raw_stream = state.raw_stream.clone();
                let state_for_js = state.clone();
                let palette_matches = palette_matches.clone();
                // A page-up/page-down step of one screenful, with a one-row overlap so context
                // carries across the jump instead of starting mid-sentence.
                let page_step = body_height.saturating_sub(1).max(1);
                move |event| {
                // Which `ScrollState` PageUp/PageDown/the mouse wheel act on depends on which
                // page is currently showing — see `AppState::trace_scroll`/`process_scroll`'s
                // doc comments for why each page needs its own.
                let active_scroll = |page: Page| match page {
                    Page::Trace => &trace_scroll,
                    Page::Process => &process_scroll,
                    Page::Inspect | Page::Chat => &scroll,
                };
                // The actual "run this" dispatch — `KeyCode::Enter` below is its main caller (a
                // fully-typed `/command args` or free-text prompt), but a no-arg command accepted
                // straight from the palette (`KeyCode::Tab`/`KeyCode::Enter if palette_open`
                // below) needs the exact same dispatch+reset without ever going through the input
                // box at all, so this is factored out rather than duplicated three times.
                let submit_command = |prompt: String| {
                    // Deliberately does *not* clear `show_welcome_overview` just because
                    // something was submitted — a submission that doesn't actually produce
                    // anything visible (a rejected shell command, a typo) used to silently kill
                    // the welcome overview anyway, even though nothing in the transcript changed.
                    // `welcome_active` (see `draw_assistant`) already gates on
                    // `messages.is_empty()` too, so the overview now naturally goes away exactly
                    // when there's something real to show instead, and `/welcome` below remains
                    // the one explicit, deliberate way to turn it off (or back on) regardless of
                    // transcript state.
                    // Switches to `Page::Inspect` and stays there — every other branch below falls
                    // through to the trailing reset-to-`Page::Chat` at the end of this closure,
                    // which is right for a one-shot command but wrong for a command whose whole
                    // point is changing which page is showing, so this returns early instead.
                    if prompt.trim() == "/inspect" {
                        *page.write() = Page::Inspect;
                        return;
                    }

                    if prompt.trim() == "/welcome" {
                        let mut show = show_welcome_overview.write();
                        *show = !*show;
                        if let Some(sender) = persistence_writes.read().clone() {
                            let _ = sender.send(PersistenceWrite::ShowWelcomeOverview(*show));
                        }
                        notify(&messages, if *show { "Welcome overview turned on." } else { "Welcome overview turned off." });
                        return;
                    }

                    if let Some(label) = prompt.strip_prefix("/task ").map(str::trim).filter(|l| !l.is_empty()) {
                        let new_task = TaskRow { label: label.to_owned(), status: "pending".into() };
                        let mut tasks = tasks.write();
                        tasks.push(new_task);

                        // Never blocks the render thread on `sqld` — see
                        // `sync_overlay_bounds_to_persistence`'s doc comment for the full
                        // "spamming inputs stalls the whole UI for a long time" story this fixes
                        // across every persistence call site, not just this one. The UI already
                        // reflects the new task; a failed save just gets logged, same as before.
                        if let Some(sender) = persistence_writes.read().clone() {
                            let _ = sender.send(PersistenceWrite::Tasks(tasks.clone()));
                        }
                    } else if let Some(url) = prompt.strip_prefix("/browser ").map(str::trim).filter(|u| !u.is_empty()) {
                        // This whole app now runs *as* a Bevy app (see `AssistantTerminalPlugin`
                        // in `main`) instead of owning its own event loop, specifically so this
                        // can be a real in-process `SceneCommand` instead of spawning a second
                        // `cargo run` process and paying its full build-freshness-check and
                        // cold-start cost on every single `/browser` call. Queued here rather than
                        // written directly — this handler runs inside `TerminalSurface::draw`'s
                        // own dispatch, not as a Bevy system, so it can't take a `MessageWriter` —
                        // and drained by `AssistantTerminalPlugin::draw_ui` once it's back in
                        // normal system context (see `AppState::pending_browser_urls`).
                        pending_browser_urls.lock().push(url.to_string());

                        notify(&messages, format!("Opening a browser tab loaded to {url} …"));
                    } else if prompt.trim() == "/scene" {
                        // See `AppState::scene_window_requested`'s own doc comment — a stub for
                        // now, just a bare window with a camera.
                        scene_window_requested.store(true, Ordering::Relaxed);
                        notify(&messages, "Opening a scene window …");
                    } else if prompt.trim() == "/relay-console" {
                        // Starts both the static page server and a real relay behind it, the
                        // first time only (see `AppState::spawn_relay_console_server`/
                        // `spawn_relay_server`'s own doc comments) — a repeat `/relay-console`
                        // just reopens the same already-running page.
                        state_for_js.spawn_relay_console_server();
                        state_for_js.spawn_relay_server();
                        let url = format!("http://127.0.0.1:{RELAY_CONSOLE_PORT}/");
                        // Not pushed to `pending_browser_urls` directly — `Deno.serve`'s own
                        // startup has real latency, and the webview navigating before it's
                        // actually listening was a real, confirmed "shows a blank page" bug. See
                        // `AppState::spawn_open_page`/`scripts/open-page.js`'s own doc comments.
                        state_for_js.spawn_open_page(url.clone(), 15_000, "relay-console".to_string());
                        notify(&messages, format!("Opening the Relay Console at {url} …"));
                    } else if prompt.trim() == "/workspace" {
                        notify(&messages, describe_workspace(&anvil_root()));
                    } else if prompt.trim() == "/shape" {
                        // See `AppState::spawn_shape_command`'s doc comment for the full picture —
                        // one Ethos-authored shape, fanned out to the terminal (`Page::Process`,
                        // switched to as soon as the background task starts), a browser tab, and
                        // Aby's Unity project. The `Tool`/`Assistant` reply (and the `Page::Process`
                        // switch) happen inside the spawned task, same as any other background
                        // command in this app — only the `User` message is recorded synchronously
                        // here, same as the JS-command branch below.
                        let user_message = ChatMessage::User(prompt.clone());
                        messages.write().push(user_message.clone());

                        if let Some(sender) = persistence_writes.read().clone() {
                            let _ = sender.send(PersistenceWrite::Message(user_message));
                        }

                        state_for_js.spawn_shape_command();
                    } else if let Some((name, args)) = parse_slash_command(&prompt)
                        && let Some(command) = commands.iter().find(|command| command.name == name)
                        && let Some(script) = &command.script
                    {
                        let user_message = ChatMessage::User(prompt.clone());
                        messages.write().push(user_message.clone());

                        if let Some(sender) = persistence_writes.read().clone() {
                            let _ = sender.send(PersistenceWrite::Message(user_message));
                        }

                        // Runs on a background thread, see `AppState::spawn_js_command`'s doc
                        // comment for why this can't block the render thread here. The reply
                        // appears a moment later, once the script actually finishes.
                        let command_name = format!("{} {}", command.name, args).trim().to_owned();
                        state_for_js.spawn_js_command(command_name, script.clone(), args.to_owned());
                    } else if prompt.trim_start().starts_with("cargo run") {
                        // `cargo run` gets its own real terminal window instead of the piped-
                        // stdout shell passthrough below — anything it launches (a Bevy app with
                        // its own raw-mode terminal UI, say) needs a genuine TTY to render
                        // correctly, which a piped subprocess can't give it. `working_dir` is
                        // `anvil_root()` (the project being worked on, see its own doc comment),
                        // the same place `cargo run -p <name>` needs to resolve a package name
                        // against — whatever Cargo workspace that project happens to be, not this
                        // dev checkout specifically.
                        let working_dir = anvil_root();
                        messages.write().push(ChatMessage::User(prompt.clone()));
                        notify(&messages, format!("Opening a terminal for `{}` …", prompt.trim()));
                        state_for_js.spawn_open_terminal(prompt.trim().to_string(), working_dir);
                    } else if !prompt.is_empty() {
                        // Anything typed that isn't a recognized slash command now runs against a
                        // real configured shell (`AppState::spawn_shell_command` — see
                        // `process::resolve_shell_backend` for how the backend is chosen) instead
                        // of just being recorded with no real agent behind it. Unlike every other
                        // branch here, the `User` message is *not* recorded synchronously — see
                        // `spawn_shell_command`'s own doc comment for why that has to wait until
                        // the shell's verdict is known.
                        state_for_js.spawn_shell_command(prompt.clone());
                    }

                    // Sending a message always jumps back to the bottom, like any chat app — and
                    // back to `Page::Chat` with nothing selected, since input always targets the
                    // chat transcript regardless of what the Body area happens to be showing (a
                    // task's detail page, or the Trace/Process firehose). Without this, submitting
                    // while looking at one of those silently acted on the hidden transcript
                    // underneath with no visible confirmation it happened at all.
                    *scroll.write() = ScrollState::Following;
                    *selected_task.write() = None;
                    *page.write() = Page::Chat;
                };
                match event {
                    CrosstermEvent::Key(key) => match key.code {
                        // A Ctrl-held character is a shortcut (Ctrl+C to copy a selection —
                        // handled in `TerminalSurface::draw` — plus whatever else in this
                        // modifier space), never literal text; typing it here too would insert
                        // a stray "c" into the input on every copy.
                        KeyCode::Char(key_char) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if key.kind != KeyEventKind::Release {
                                let mut input = user_input.write();
                                input.push(key_char);
                                *autocomplete_index.write() = 0;

                                // Deliberately does *not* react to what's typed matching a known
                                // command name on its own — an earlier version briefly did
                                // exactly that (no-arg commands ran immediately, arg-taking ones
                                // got an auto-appended trailing space), and live use found it both
                                // jarring (the cursor jumping ahead unasked for arg-taking
                                // commands) and actively dangerous (a no-arg command executing the
                                // instant its name was fully typed, with no explicit confirming
                                // action at all — a stray extra keystroke while typing something
                                // else entirely could fire a real command). Removed; Tab (accept
                                // the highlighted autocomplete suggestion) and Enter (submit) are
                                // the only two ways anything here runs, both requiring the user to
                                // actually press them.
                            }
                        }
                        KeyCode::Backspace => {
                            if key.kind != KeyEventKind::Release {
                                user_input.write().pop();
                                *autocomplete_index.write() = 0;
                            }
                        }
                        // Whenever the palette's showing at all — mid-autocomplete, or just
                        // defaulted open alongside the welcome message with nothing typed yet,
                        // see `draw_assistant`'s `palette_open` — Up/Down navigate its suggestion
                        // list. Otherwise they navigate the task list instead, swapping the Body
                        // area to that task's own page — see the second match arm below. The two
                        // can't both apply at once (the palette being open is never also a valid
                        // moment to be browsing tasks), so there's no ambiguity about which Up/Down
                        // means.
                        KeyCode::Up if palette_interactive => {
                            if key.kind != KeyEventKind::Release {
                                let mut index = autocomplete_index.write();
                                *index = index.checked_sub(1).unwrap_or(palette_matches.len() - 1);
                            }
                        }
                        KeyCode::Down if palette_interactive => {
                            if key.kind != KeyEventKind::Release {
                                let mut index = autocomplete_index.write();
                                *index = (*index + 1) % palette_matches.len();
                            }
                        }
                        // Wraps through `None` (the transcript) on both ends, rather than
                        // clamping at the first/last task — that's what makes "go back to the
                        // transcript" reachable with the same two keys instead of a separate
                        // binding. A no-op with no tasks at all (nothing to select).
                        KeyCode::Up if !tasks.read().is_empty() => {
                            if key.kind != KeyEventKind::Release {
                                let task_count = tasks.read().len();
                                let mut selected = selected_task.write();
                                *selected = match *selected {
                                    None => Some(task_count - 1),
                                    Some(0) => None,
                                    Some(index) => Some(index - 1),
                                };
                                drop(selected);
                                *scroll.write() = ScrollState::Following;
                            }
                        }
                        KeyCode::Down if !tasks.read().is_empty() => {
                            if key.kind != KeyEventKind::Release {
                                let task_count = tasks.read().len();
                                let mut selected = selected_task.write();
                                *selected = match *selected {
                                    None => Some(0),
                                    Some(index) if index + 1 == task_count => None,
                                    Some(index) => Some(index + 1),
                                };
                                drop(selected);
                                *scroll.write() = ScrollState::Following;
                            }
                        }
                        // Cycles the selected task's status (pending → running → done, or the
                        // reverse) — see `cycle_selected_task_status`'s doc comment for the
                        // persistence side of this. Left/Right rather than Enter/Space: neither
                        // means anything else right now, and — unlike Up/Down, already repurposed
                        // from cursor movement this input doesn't support in the first place —
                        // this specifically needs a key that's never a valid character to type,
                        // so it can never collide with composing a message. Guarded on
                        // `!palette_interactive` too, same as Up/Down's own task-navigation arms
                        // above — a task can stay selected while the palette is separately
                        // interactive (nothing clears `selected_task` when typing starts, or when
                        // the welcome message defaults the palette open), and without this guard
                        // Left/Right would silently mutate the selected task's status while the
                        // user's actual attention is
                        // on picking a command. Only live while a task is selected and the palette
                        // isn't showing; otherwise falls through, e.g. to normal text entry.
                        // Cycles `Page::Inspect`'s subpage — checked before the task-status arms
                        // below so it takes priority whenever `Page::Inspect` is showing (the two
                        // never apply at once in practice, but this makes the precedence explicit
                        // rather than accidental match-order luck).
                        KeyCode::Left if *page.read() == Page::Inspect => {
                            if key.kind != KeyEventKind::Release {
                                let mut subpage = inspect_subpage.write();
                                let index = InspectSubpage::ALL.iter().position(|s| s == &*subpage).unwrap_or(0);
                                *subpage = InspectSubpage::ALL[(index + InspectSubpage::ALL.len() - 1) % InspectSubpage::ALL.len()];
                            }
                        }
                        KeyCode::Right if *page.read() == Page::Inspect => {
                            if key.kind != KeyEventKind::Release {
                                let mut subpage = inspect_subpage.write();
                                let index = InspectSubpage::ALL.iter().position(|s| s == &*subpage).unwrap_or(0);
                                *subpage = InspectSubpage::ALL[(index + 1) % InspectSubpage::ALL.len()];
                            }
                        }
                        KeyCode::Left if selected_task.read().is_some() && !palette_interactive => {
                            if key.kind != KeyEventKind::Release {
                                cycle_selected_task_status(&tasks, &selected_task, &persistence_writes, false);
                            }
                        }
                        KeyCode::Right if selected_task.read().is_some() && !palette_interactive => {
                            if key.kind != KeyEventKind::Release {
                                cycle_selected_task_status(&tasks, &selected_task, &persistence_writes, true);
                            }
                        }
                        // Accepts the highlighted suggestion whenever the palette's showing at
                        // all — mid-autocomplete, or just defaulted open with nothing typed yet;
                        // otherwise Tab keeps its existing job of toggling tool-call detail. A
                        // command that takes no args (`args_hint.is_empty()` — see `SlashCommand`'s
                        // own doc comment) has nothing left to type, so accepting it submits
                        // directly instead of completing to "/name " and waiting for a second
                        // Enter that would just submit empty args anyway — only a command that
                        // actually needs args should "jump ahead" to typing them.
                        KeyCode::Tab => {
                            if key.kind != KeyEventKind::Release {
                                if palette_interactive {
                                    if let Some(command) = palette_matches.get(palette_selected_index) {
                                        if command.args_hint.is_empty() {
                                            user_input.write().clear();
                                            *autocomplete_index.write() = 0;
                                            submit_command(format!("/{}", command.name));
                                        } else {
                                            *user_input.write() = format!("/{} ", command.name);
                                            *autocomplete_index.write() = 0;
                                        }
                                    }
                                } else {
                                    let mut expanded_flag = expanded_flag.write();
                                    *expanded_flag = !*expanded_flag;
                                }
                            }
                        }
                        // Leaves the TUI entirely for `RawStreamGate`'s plain trace stream — a
                        // different axis from F2/F3 below (those swap *within* the TUI's own Body
                        // area; this leaves the TUI, `Scaffold`/`TerminalSurface` and all). F5, not
                        // F1 — F1 is reserved for help/settings, and this is
                        // a secondary tool (`Page::Trace`/F2, nested inside the normal UI, is the
                        // first thing to reach for). Only ever turns it on from here; turning it
                        // back off happens in `assistant_terminal_draw`'s own raw, non-`Scaffold`
                        // poll loop (`run_raw_stream_tick`) once that mode is actually active — the
                        // same "own its own input while active" split `spawn_input_watcher` uses
                        // elsewhere in this file.
                        KeyCode::F(5) => {
                            if key.kind != KeyEventKind::Release {
                                raw_stream.store(true, Ordering::Relaxed);
                            }
                        }
                        // Toggles the Body area between the chat transcript and the raw tracing
                        // firehose (see `Page`/`LineBuffer`) — F2/F3 rather than one of the keys
                        // already in use above (Tab/Up/Down/PageUp/PageDown/Enter all mean
                        // something else already, several of them context-dependent). Each
                        // toggles independently against `Page::Chat` — pressing F2 while on
                        // `Page::Process` switches to `Page::Trace` (not back to `Process`),
                        // and vice versa for F3, so either key always lands on its own page from
                        // anywhere rather than needing two presses.
                        // F1 deliberately unbound — reserved for this app's own terminal
                        // help/settings screen (not yet built), the near-universal convention
                        // it's meant for.
                        KeyCode::F(2) => {
                            if key.kind != KeyEventKind::Release {
                                let mut page = page.write();
                                *page = if *page == Page::Trace { Page::Chat } else { Page::Trace };
                            }
                        }
                        KeyCode::F(3) => {
                            if key.kind != KeyEventKind::Release {
                                let mut page = page.write();
                                *page = if *page == Page::Process { Page::Chat } else { Page::Process };
                            }
                        }
                        // One consistent "back" key for every one of this app's non-Chat modes —
                        // Trace/Process (F2/F3) and a selected task (Up/Down) are otherwise each
                        // only reachable and dismissable through their own separate key, which
                        // made it hard to tell "what got me here, and how do I leave" at a glance.
                        // Esc always means the same thing regardless of which of those got you off
                        // `Page::Chat`. At the root already (nothing left to back out of), Esc
                        // instead arms the quit confirmation below rather than doing nothing.
                        KeyCode::Esc => {
                            if key.kind != KeyEventKind::Release {
                                let at_root = *page.read() == Page::Chat && selected_task.read().is_none();
                                if at_root {
                                    let confirmed = exit_warned_since.read().is_some_and(|since| since.elapsed() < EXIT_CONFIRM_WINDOW);
                                    if confirmed {
                                        quit_requested.store(true, Ordering::Relaxed);
                                    } else {
                                        *exit_warned_since.write() = Some(Instant::now());
                                    }
                                } else {
                                    *selected_task.write() = None;
                                    *page.write() = Page::Chat;
                                }
                            }
                        }
                        KeyCode::PageUp => {
                            if key.kind != KeyEventKind::Release {
                                scroll_up(active_scroll(*page.read()), natural_offset, page_step);
                            }
                        }
                        KeyCode::PageDown => {
                            if key.kind != KeyEventKind::Release {
                                scroll_down(active_scroll(*page.read()), natural_offset, page_step);
                            }
                        }
                        KeyCode::Enter if palette_interactive => {
                            // Same as Tab whenever the palette's showing — accepts the highlighted
                            // suggestion (running it directly if it takes no args, same reasoning
                            // as Tab above) instead of submitting whatever's in the input box,
                            // which is exactly how a user with nothing typed yet picks a command
                            // by Up/Down-ing to it and hitting Enter, same as if they'd typed its
                            // name out.
                            if key.kind != KeyEventKind::Release
                                && let Some(command) = palette_matches.get(palette_selected_index)
                            {
                                if command.args_hint.is_empty() {
                                    user_input.write().clear();
                                    *autocomplete_index.write() = 0;
                                    submit_command(format!("/{}", command.name));
                                } else {
                                    *user_input.write() = format!("/{} ", command.name);
                                    *autocomplete_index.write() = 0;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if key.kind != KeyEventKind::Release {
                                let mut user_input = user_input.write();
                                let prompt = user_input.trim().to_owned();
                                user_input.clear();
                                drop(user_input);

                                submit_command(prompt);
                            }
                        }
                        _key_code => {}
                    }
                    // Mouse capture is already enabled by the terminal runtime (see
                    // `app.rs`'s `EnableMouseCapture`), so the wheel reaches us directly rather
                    // than going to the terminal emulator's own scrollback — a more reliable
                    // scroll input than PageUp/PageDown on terminals/multiplexers that bind
                    // those to their own scrollback instead of passing them through.
                    CrosstermEvent::Mouse(mouse_event) => {
                        let active_scroll = active_scroll(*page.read());
                        match mouse_event.kind {
                            MouseEventKind::ScrollUp => scroll_up(active_scroll, natural_offset, MOUSE_SCROLL_STEP),
                            MouseEventKind::ScrollDown => scroll_down(active_scroll, natural_offset, MOUSE_SCROLL_STEP),
                            _ => {}
                        }
                    }
                    // Bracketed paste (`app.rs`'s `EnableBracketedPaste`) delivers the whole
                    // pasted string as one event, unlike typed input, which arrives one `Key`
                    // at a time.
                    CrosstermEvent::Paste(text) => {
                        user_input.write().push_str(&text);
                        *autocomplete_index.write() = 0;
                    }
                    _event => {}
                }
                }
            })
            .slot::<Body>(|body| {
                body
                    // Lines up the transcript's left edge with the Footer's input, which sits
                    // 1 column in from its own border. `Padding::top` is a scroll-safe way to add
                    // breathing room under the Header — `Overflow::Scroll` below means a row
                    // temporarily off-screen because of it is still reachable by scrolling, never
                    // actually clipped.
                    .style(Padding::left(1))
                    .style(Padding::right(1))
                    .style(Padding::top(1))
                    .style(Overflow::Scroll)
                    .style(ScrollPosition::new(scroll_offset))
                    .content(Some(body_content))
            })
            // A small dynamic bar showing `/command` matches while one's being typed, sitting
            // directly above the input it's completing — easier to spot right where you're
            // typing than the old shared-overlay dropdown in the top-right corner. Zero height
            // (and no content) when not autocompleting, so it takes up no space the rest of the
            // time; `autocomplete_bar_height` (computed above, alongside `body_height`) has to
            // stay in sync with the row count `autocomplete_bar_text` actually renders.
            .slot::<AutocompleteBar>(|bar| {
                bar.style(Size::height(autocomplete_bar_height))
                    .style(Padding::left(1))
                    .style(Padding::right(1))
                    .style(Padding::top(1))
                    .content(palette_open.then(|| autocomplete_bar_text(&palette_matches, Some(palette_selected_index), &state.welcome_footer)))
            })
            .slot::<InputGap>(|gap| gap.style(Size::height(INPUT_GAP_HEIGHT)))
            .slot::<Footer>(|footer| {
                // A steady 530ms on/off cadence — the common terminal-emulator cursor blink
                // rate (iTerm2/Terminal.app's default); a text-input caret reads as "blinking",
                // not "glowing".
                let cursor_visible = (state.start.elapsed().as_millis() / 530) % 2 == 0;

                footer
                    .style(FlexDirection::Row)
                    .style(Size::height(FOOTER_HEIGHT))
                    .style(Border::new(1, BorderStyle::Solid, Some(Color::new(ACCENT_ORANGE.0, ACCENT_ORANGE.1, ACCENT_ORANGE.2, 255))))
                    .element(Input::<String>::new(input_display.clone()).with_cursor_visible(cursor_visible))
            })
            // Model/turn/keybinding info lives below the input now — easier to spot right next
            // to where you're actually typing than tucked into the header above the transcript.
            .slot::<StatusLine>(|status| {
                // Overrides the regular status text while a copy-selection problem looks real
                // (see `AppState::mouse_trouble_since`), decaying back to normal on its own once
                // either a copy succeeds or the problem goes stale (`MOUSE_HINT_MAX_AGE`).
                let mouse_hint = state.mouse_trouble_since.read().and_then(|since| {
                    let elapsed = since.elapsed();
                    if elapsed > MOUSE_HINT_MAX_AGE {
                        None
                    } else if elapsed < MOUSE_HINT_DELAY {
                        Some("Nothing selected to copy.")
                    } else {
                        Some("Still nothing to copy. Hold Option (⌥) while dragging to select text natively.")
                    }
                });

                // Top priority: a command in flight (see `RunningCommand`'s doc comment) — a
                // live spinner, elapsed time, and a peek at its most recent output line, without
                // switching away from whatever page is already showing. Replaces the earlier
                // "force-switch to Page::Process on every command" behavior — a whole page switch
                // per command is overkill, but a genuinely slow command should still visibly show
                // progress rather than leaving the user wondering if it stalled.
                let running_hint = state.running_command.read().as_ref().map(|running| {
                    let spinner = SPINNER_FRAMES[(state.start.elapsed().as_millis() / 80) as usize % SPINNER_FRAMES.len()];
                    let elapsed = running.started_at.elapsed().as_secs_f64();
                    let peek = state.process_buffer.last_line().unwrap_or_default();
                    format!(
                        "{} {} — {elapsed:.1}s │ {peek} {}",
                        spinner.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2),
                        running.label,
                        "(F3 for output)".truecolor(DIM.0, DIM.1, DIM.2),
                    )
                });

                // A quiet, fast-decaying nudge that the shell fallback saw the last input and
                // declined it outright (see `AppState::shell_rejected_since` and
                // `process::ShellOutcome::Rejected`) — expected to fire often during ordinary use
                // (typos, stray text), so unlike `mouse_hint` above it never escalates, just
                // disappears after `SHELL_REJECTED_HINT_DURATION`.
                let shell_hint = state.shell_rejected_since.read().and_then(|since| (since.elapsed() < SHELL_REJECTED_HINT_DURATION).then_some("shell said nah"));

                // A brief warning after the first Esc at the root — see `AppState::
                // exit_warned_since`'s own doc comment. High priority while it's live: it's a
                // direct consequence of something the user just pressed, not background state.
                let exit_hint = state.exit_warned_since.read().and_then(|since| (since.elapsed() < EXIT_CONFIRM_WINDOW).then_some("Press Esc again to quit"));

                // Only shown once there's actually somewhere to back out of (Trace/Process, or a
                // selected task) — Esc is a no-op on the plain chat transcript, so advertising it
                // there would just be noise. Same header/Esc pairing as the mode indicator above:
                // this is the "how do I leave" half, that's the "where am I" half.
                let show_back_hint = page != Page::Chat || selected_task.is_some();

                status
                    .style(Size::height(STATUS_HEIGHT))
                    .style(Padding::left(1))
                    .style(FontStyle::Italic)
                    .style(ContentColor::new(DIM.0, DIM.1, DIM.2, 255))
                    .content(Some(match (running_hint, exit_hint, mouse_hint, shell_hint) {
                        (Some(running), _, _, _) => running,
                        // Dim, not orange — this is routine navigation (Esc-to-quit is always
                        // two presses by design, never a mistake needing real attention), unlike
                        // the mouse-trouble hint below it, which is an actual warning.
                        (None, Some(hint), _, _) => format!("{}", hint.truecolor(DIM.0, DIM.1, DIM.2)),
                        (None, None, Some(hint), _) => format!("{}", hint.truecolor(RED.0, RED.1, RED.2)),
                        (None, None, None, Some(hint)) => format!("{}", hint.truecolor(DIM.0, DIM.1, DIM.2)),
                        // "What can or should I do right now", nothing else — page toggles,
                        // fps/cwd, and the `/inspect` pointer are reference material or passive
                        // telemetry, not something to scan for moment to moment while typing.
                        // The truly idle case (nothing else to say) used to render as a blank
                        // line — that dead space should earn its keep instead.
                        // `/ for commands` is the one static, always-true fact worth a glance
                        // here, and only shown when the palette isn't already up making the same
                        // point live (see `palette_open`).
                        (None, None, None, None) => [
                            show_back_hint.then(|| "Esc: back".to_string()),
                            is_scrolled_up.then(|| format!("{}", "↑ scrolled (PgDn to catch up)".truecolor(ACCENT_ORANGE.0, ACCENT_ORANGE.1, ACCENT_ORANGE.2))),
                            (!palette_open).then(|| format!("{}", "/ for commands".truecolor(DIM.0, DIM.1, DIM.2))),
                            // The idle status line used to stop at the line above, leaving the
                            // F-keys (a real, already-bound way to navigate this app) completely
                            // undiscoverable unless someone already knew to press one — this dead
                            // space is the one place with nothing better to show, so it surfaces
                            // them here instead. Not gated on `palette_open` like `/ for commands`
                            // is — the palette never advertises these, so there's nothing for it
                            // to duplicate.
                            Some(format!("{}", "F2 trace · F3 process · F5 raw stream".truecolor(DIM.0, DIM.1, DIM.2))),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · "),
                    }))
            });

        // Only shown once there's actually a task to show — an empty list is nothing worth a
        // permanent fixture on screen. A floating window layered over the transcript instead of
        // taking up its own row — `overlay` renders a detached scaffold at a fixed corner (see
        // `TerminalSurface::overlay_rect`) rather than partitioning space like a slot does.
        if task_count == 0 {
            root
        } else {
            root.overlay(|overlay| {
                overlay
                    .style(Size(OVERLAY_WIDTH.into(), overlay_height.into(), Value::Auto))
                    // Keep clear of the Footer bar and the status line below it — the overlay's
                    // positioning has no idea the root layout put those there.
                    .style(OverlayInset::bottom(INPUT_GAP_HEIGHT + FOOTER_HEIGHT + STATUS_HEIGHT + 1))
                    // ...and clear of the Body's scrollbar on the right, for the same reason —
                    // the default 1-cell inset alone sits in the same column as the scrollbar.
                    .style(OverlayInset::right(2))
                    // Dotted and dim rather than the header/footer's own solid accent borders —
                    // a background fixture that's easy to check at a glance, not a focal point.
                    .style(Border::new(1, BorderStyle::Dotted, Some(Color::new(DIM.0, DIM.1, DIM.2, 255))))
                    .style(BackgroundColor::new(BACKGROUND.0, BACKGROUND.1, BACKGROUND.2, 255))
                    // "0 1" — no vertical padding (the border alone gives enough breathing
                    // room top/bottom), 1 cell horizontal so text doesn't touch the border.
                    .style(Padding::left(1))
                    .style(Padding::right(1))
                    .content(Some(state.tasks_overlay_text(selected_task_index)))
            })
        }
    }, Duration::ZERO)?;

    // A real signal, not a guess (see `TerminalAction::EmptyCopyAttempt`'s doc comment): only
    // set on an actual failed copy, only cleared on an actual successful one.
    match action {
        TerminalAction::EmptyCopyAttempt => {
            if state.mouse_trouble_since.read().is_none() {
                *state.mouse_trouble_since.write() = Some(Instant::now());
            }
        }
        TerminalAction::Copied => *state.mouse_trouble_since.write() = None,
        _ => {}
    }

    Ok(action)
}

//---
struct StatusLine;

/// The slot rendered directly above the `Footer` input while a `/command` name is being typed —
/// holds what used to share the top-right overlay with the tasks list (see `draw_assistant`'s
/// `autocomplete_bar_height`/`autocomplete_bar_text`). A dedicated slot rather than reusing
/// `overlay` because `Scaffold` only supports one detached overlay at a time (see
/// `overlay`'s doc comment in `escher_core::scaffold`), and the tasks overlay needed to keep
/// showing full time instead of swapping content.
struct AutocompleteBar;

/// A blank, contentless spacer slot between the `AutocompleteBar`/`Body` above and the `Footer`
/// input below — see `INPUT_GAP_HEIGHT`.
struct InputGap;
