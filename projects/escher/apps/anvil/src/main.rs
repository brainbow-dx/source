extern crate alloc;

use std::process::Command;
use std::process::ExitCode;
use std::process::Stdio;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
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

use unicode_width::UnicodeWidthChar;
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

// `/scene` opens its window *in this same process* — see `AssistantTerminalPlugin`'s own doc
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
use bevy::window::RawHandleWrapper;

use escher_bevy::os::OsPlugin;
use escher_bevy::terminal::spawn_input_watcher;
use escher_bevy::webview::SceneCommand;
use escher_bevy::EscherBevyConfig;
use escher_bevy::EscherBevyPlugin;

use escher_appkit::bevy::{TabInfo, TabStripEvent, TabStripState, ThemeState, ToolbarEvent, ToolbarPlugin, ToolbarState, ToolbarSystems, ToolbarTheme, WantsTabStrip, WantsToolbar, TOOLBAR_HEIGHT};

// Anvil: an inventor's notebook built entirely out of Escher scaffolds — an AI-assistant-style
// terminal UI (a scrollable transcript of user/assistant/tool turns; PageUp/PageDown, not the
// terminal emulator's own scrollback, which generally doesn't work for a raw-mode/redrawing TUI —
// the app owns its own scroll position instead) above a bordered input prompt, with a real native
// webview + chrome bar living alongside it in the same process (`/scene <url>`). Doubles as a
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
    /// A handful of dependencies are silenced below the app's own default — at plain `trace`,
    /// `wgpu_core`/`wgpu_hal` log every single GPU call (`Device::create_bind_group`,
    /// `Queue::submit`, ...), `naga` dumps its full numeric-overload-resolution rule table on
    /// every shader type-check, `bevy_shader` logs every shader-def permutation it processes,
    /// `winit` traces every single AppKit window-delegate callback, and `hyper`/`libsql_sys`/
    /// `libsql_replication` trace every byte of the `sqld` replication connection's HTTP/WAL
    /// traffic — all every frame or every request, `wgpu`/`naga` the same `"wgpu=error,naga=warn"`
    /// suppression Bevy's own `LogPlugin` recommends by default, the rest found the same way:
    /// watching `anvil.log` grow to 11MB+ (~210k lines) within minutes of normal use, then
    /// confirming what was left after the first pass by watching the raw stream (F1/`--no-tui`)
    /// live. None of it is app-level tracing.
    #[arg(
        short,
        long,
        default_value = "trace,wgpu=error,naga=warn,bevy_shader=warn,winit=warn,hyper=warn,libsql_sys=warn,libsql_replication=warn"
    )]
    log_level: String,

    /// Skip the TUI at startup and just print the raw, unformatted trace stream straight to the
    /// terminal instead — the same thing F1 switches to at runtime (see `RawStreamGate`), useful
    /// when a `Scaffold`/`TerminalSurface` bug means the TUI itself can't be trusted to render.
    #[arg(long, default_value_t = false)]
    no_tui: bool,

    /// Print this run's captured trace output (from `anvil.log`) to stdout after exiting.
    /// Otherwise the only way to see what happened during a run is `tail -f anvil.log` in a
    /// second terminal while it's still open — the alternate screen this app draws to hides its
    /// own stdout, and the log file's own content stops being reachable once the process exits.
    #[arg(long, default_value_t = false)]
    dump_trace: bool,

    /// Wipes all persisted messages/tasks from `sqld` and exits — doesn't launch the TUI.
    #[arg(long, default_value_t = false)]
    reset_data: bool,
}

/// Forwards `tracing::*` events into the transcript live, but only the ones that happen inside
/// an active `live_trace` span (see `spawn_js_command`) — ambient background noise (the render
/// loop, idle persistence chatter) never has that span active, so it never reaches here. This is
/// the "relevant to the current running/focused task" filter: a command's own `live_trace` span
/// is exactly the definition of "currently running," so anything logged while it's entered is,
/// by construction, about that task.
struct TranscriptLayer {
    sender: mpsc::Sender<String>,
}

impl<S> tracing_subscriber::Layer<S> for TranscriptLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let in_live_trace = ctx
            .event_scope(event)
            .is_some_and(|scope| scope.from_root().any(|span| span.name() == "live_trace"));

        if !in_live_trace {
            return;
        }

        let mut message = MessageVisitor::default();
        event.record(&mut message);

        let _ = self.sender.send(format!("{} {}", event.metadata().level(), message.text));
    }
}

#[derive(Default)]
struct MessageVisitor {
    text: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.text = format!("{value:?}");
        }
    }
}

/// How many lines a `LineBuffer` retains before dropping the oldest — these run for the whole
/// process lifetime (`trace_buffer` fed by every single `tracing::*` call, including chatty
/// libsql/sqld internals; `process_buffer` fed by every subprocess run for the rest of the
/// session), so an unbounded buffer would leak memory over a long session.
const LINE_BUFFER_CAPACITY: usize = 2000;

/// A bounded, thread-safe ring buffer of already-formatted lines (ANSI codes included, kept
/// verbatim rather than stripped), oldest dropped once full. Backs two independent, differently-
/// fed pages (see `Page`): `AppState::trace_buffer` (`Page::Trace`), fed by `tracing_subscriber`
/// via `LineBuffer`'s own `MakeWriter` impl below — exactly what would print to a plain
/// terminal's stdout if this app weren't drawing its own alternate-screen UI over it, the same
/// unscoped "everything" feed `file_layer` writes to `anvil.log` (unlike `TranscriptLayer`,
/// which only forwards events inside an active `live_trace` span) — and `AppState::
/// process_buffer` (`Page::Process`), fed directly (not through `tracing` at all) by
/// `run_js_command`'s own stdout/stderr reader, line for line, completely unannotated — a genuine
/// raw subprocess stdio view, not a paraphrase of it through a log line.
#[derive(Clone)]
struct LineBuffer {
    lines: Arc<Mutex<VecDeque<String>>>,
}

impl LineBuffer {
    fn new() -> Self {
        LineBuffer { lines: Arc::new(Mutex::new(VecDeque::with_capacity(LINE_BUFFER_CAPACITY))) }
    }

    /// Appends one already-formatted (ANSI included) line, dropping the oldest once at capacity.
    fn push_line(&self, line: String) {
        let mut lines = self.lines.lock();
        if lines.len() >= LINE_BUFFER_CAPACITY {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    /// A snapshot of everything currently retained, oldest first, joined back into one string —
    /// ready to hand straight to `.with_content` the same way `build_transcript` does for the
    /// chat page; the terminal surface parses embedded ANSI out of any content string via
    /// `ansi_to_tui` regardless of which node it came from (see `TerminalSurface::draw`), so no
    /// separate `ansi_to_tui` call is needed here.
    fn snapshot(&self) -> String {
        self.lines.lock().iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

/// A line-buffering `io::Write` sink for `tracing_subscriber::fmt::layer()`'s writer — the fmt
/// layer may split one event's output across more than one `write()` call (formatted fields,
/// then a trailing newline, as separate calls in some configurations), so this can't assume one
/// `write()` is one complete line. Bytes accumulate in `pending` until a `\n` completes a line,
/// at which point that whole line (ANSI codes included) is pushed into the shared `LineBuffer`.
/// Only `AppState::trace_buffer` actually goes through this — `process_buffer` is fed directly
/// (see `LineBuffer`'s own doc comment), bypassing `tracing`/this writer entirely.
struct LineBufferWriter {
    buffer: LineBuffer,
    pending: String,
}

impl io::Write for LineBufferWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pending.push_str(&String::from_utf8_lossy(bytes));
        while let Some(newline_at) = self.pending.find('\n') {
            let line = self.pending[..newline_at].to_string();
            self.buffer.push_line(line);
            self.pending.drain(..=newline_at);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LineBuffer {
    type Writer = LineBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LineBufferWriter { buffer: self.clone(), pending: String::new() }
    }
}

/// Backs the plain, non-TUI "raw trace stream" mode (`--no-tui` at startup, or F1 to switch to
/// it live) — the same unscoped firehose `file_layer`/`trace_page_layer` already see, printed as
/// ordinary scrolling terminal text instead of through any `Scaffold`/`TerminalSurface`
/// rendering. Writes straight to real stdout while `active` is true, discards otherwise — shares
/// the exact `Arc<AtomicBool>` `AppState::raw_stream` uses (set by the F1 handler, read by
/// `assistant_terminal_draw`), so flipping that one flag takes effect on the very next tracing
/// event with no other plumbing, and this mode keeps printing even if something in the
/// `Scaffold`/dispatch code that draws the TUI is broken, since none of that code runs to
/// produce this output.
#[derive(Clone)]
struct RawStreamGate {
    active: Arc<AtomicBool>,
}

impl io::Write for RawStreamGate {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.active.load(Ordering::Relaxed) {
            // Raw mode disables the terminal's automatic `\n` -> `\r\n` translation (ONLCR), so
            // without this every line would stair-step one column further right than the last.
            let mut stdout = io::stdout();
            for chunk in bytes.split_inclusive(|&byte| byte == b'\n') {
                match chunk.split_last() {
                    Some((b'\n', rest)) => {
                        stdout.write_all(rest)?;
                        stdout.write_all(b"\r\n")?;
                    }
                    _ => stdout.write_all(chunk)?,
                }
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.active.load(Ordering::Relaxed) {
            io::stdout().flush()?;
        }
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RawStreamGate {
    type Writer = RawStreamGate;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

//---
/// Hosts this whole app's terminal UI *inside* a Bevy app, instead of `TerminalApp::run` owning
/// its own event loop — the only way `/scene` can open a real in-process window (see `main`):
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
    let mut surface =
        TerminalSurface::<CrosstermBackend<Stdout>>::try_default().expect("failed to construct the terminal surface");

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
/// bare F1 press (raw mode stays enabled the whole time this app runs, so it arrives without
/// Enter) to hand control back to the TUI.
fn run_raw_stream_tick(terminal: &mut TerminalHandle, state: &AppState) {
    if !terminal.in_raw_stream {
        terminal.in_raw_stream = true;
        let _ = crossterm::execute!(terminal.surface.backend_mut(), crossterm::terminal::LeaveAlternateScreen);
        print_raw_stream_banner();
    }

    if let Ok(true) = crossterm::event::poll(Duration::from_millis(33))
        && let Ok(CrosstermEvent::Key(key)) = crossterm::event::read()
        && key.kind != KeyEventKind::Release
        && key.code == KeyCode::F(1)
    {
        state.raw_stream.store(false, Ordering::Relaxed);
    }
}

fn print_raw_stream_banner() {
    let _ = io::stdout().write_all(b"\r\n-- raw trace stream: press F1 to switch to the TUI --\r\n\r\n");
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
    mut scene_evt: MessageWriter<SceneCommand>,
) {
    #[cfg(unix)]
    if terminal.signal_flag.load(std::sync::atomic::Ordering::Relaxed) != 0 {
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

    // Drain every already-pending terminal event this tick, not just one. `TerminalSurface::draw`
    // (via `draw_with_poll_timeout(.., Duration::ZERO)`, see that method's own doc comment) renders
    // the *current* state, then does one non-blocking poll+dispatch of a single event if one's
    // already waiting — mouse-based text selection needs the just-rendered frame buffer to resolve
    // a click to a character, so render has to come before dispatch (see `surface.rs`'s own
    // comment on this). Hosted here as a Bevy `PreUpdate` system, this only runs once per wake —
    // typically once per burst of input (`spawn_input_watcher`), not once per keystroke — so a
    // single draw call only ever clears one keystroke of a fast typing burst per tick, and nothing
    // else prompts another tick until the *next* real event arrives: `spawn_input_watcher`'s own
    // drain-wait loop only re-wakes Bevy when the OS input buffer isn't empty yet, so if a tick
    // stops short of draining it, the burst just stalls until something else happens to nudge
    // Bevy again (or the 5s/60s `WinitSettings::desktop_app()` fallback).
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
    const MAX_DRAWS_PER_TICK: u32 = 64;

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

    for url in state.pending_scenes.lock().drain(..) {
        scene_evt.write(SceneCommand { url });
    }
}

fn assistant_terminal_exit(terminal: Option<ResMut<TerminalHandle>>, mut exit_evt: MessageReader<AppExit>) {
    let Some(mut terminal) = terminal else { return };

    for _ in exit_evt.read() {
        restore_assistant_terminal(&mut terminal.surface);

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

    if let Err(error) = crossterm::terminal::disable_raw_mode() {
        eprintln!("Failed to disable raw mode: {error}");
    }

    let _ = crossterm::execute!(surface.backend_mut(), crossterm::style::ResetColor);
    let _ = std::io::Write::write_all(&mut std::io::stdout(), b"\x1b[r\x1b[0m");
    let _ = std::io::stdout().flush();

    if let Err(error) = crossterm::execute!(surface.backend_mut(), crossterm::terminal::LeaveAlternateScreen) {
        eprintln!("Failed to leave alternate screen: {error}");
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
    /// stays with the tab instead of toolbar code reaching into `TabWebViews` on its own.
    loading: bool,
}

/// Every open tab for the single browser window `/scene` opens into now — supersedes the earlier
/// one-OS-window-per-`/scene` design (see `escher/spec/.agents/changelog.md`'s matching entry for
/// why). Each tab gets its own `WebView` (see `TabWebViews`) so switching tabs shows/hides the
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

/// A `url`'s host, for favicon lookup/display — plain string splitting, not a real URL parser
/// (no crate in this workspace already pulls one in for something this small); good enough for
/// the `scheme://host/path` shape every URL typed into the address bar actually has.
fn host_of(url: &str) -> String {
    url.split("://").nth(1).and_then(|rest| rest.split('/').next()).unwrap_or(url).to_string()
}

/// Every tab's own `WebView`, keyed by `Tab::id` — not `escher_bevy::webview::WebViewHandles`
/// (that's one-per-*window*, the right shape for the old one-OS-window-per-`/scene` design, wrong
/// for one window hosting many tabs). `NonSend` for the same reason `WebViewHandles` is: a
/// `WebView` wraps a native AppKit object.
#[derive(Default)]
struct TabWebViews(std::collections::HashMap<u64, escher_webview::WebView>);

/// Attaches a `WebView` for any tab that doesn't have one yet — runs every tick, picking up tabs
/// added by `open_tab` (via `/scene` or the "+ New Tab" button) the moment the browser window's
/// native handle exists. Every tab's webview shares the same insets (toolbar height, tab strip
/// width); only the active one is left visible, so a newly-opened tab starts hidden unless it's
/// also the one that just became active.
fn attach_pending_tab_webviews(mut webviews: NonSendMut<TabWebViews>, browser: Res<BrowserState>, tab_strip: Res<TabStripState>, window_query: Query<&RawHandleWrapper>) {
    let Some(window) = browser.window else { return };
    let Ok(raw_handle) = window_query.get(window) else { return };

    for tab in &browser.tabs {
        if webviews.0.contains_key(&tab.id) {
            continue;
        }

        match escher_webview::WebView::attach(
            raw_handle.get_window_handle(),
            &tab.url,
            TOOLBAR_HEIGHT,
            tab_strip.effective_width(),
            Some(escher_webview::DEFAULT_USER_AGENT),
        ) {
            Ok(webview) => {
                webview.set_hidden(browser.active != Some(tab.id));
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
fn sync_tab_loading_state(mut browser: ResMut<BrowserState>, webviews: NonSend<TabWebViews>) {
    for tab in &mut browser.tabs {
        tab.loading = webviews.0.get(&tab.id).map(|webview| webview.is_loading()).unwrap_or(false);
    }
}

/// Pushes `BrowserState`'s current shape into `ToolbarState`/`TabStripState` every tick, just
/// before `ToolbarSystems` redraws from them — the one place anvil's own tab bookkeeping and
/// `escher_appkit::bevy`'s neutral display state meet.
fn sync_toolbar_state(browser: Res<BrowserState>, mut toolbar: ResMut<ToolbarState>, mut tab_strip: ResMut<TabStripState>) {
    toolbar.address = browser.active_tab().map(|tab| tab.url.clone()).unwrap_or_default();
    toolbar.loading = browser.active_tab().map(|tab| tab.loading).unwrap_or(false);
    tab_strip.tabs = browser.tabs.iter().map(|tab| TabInfo { id: tab.id, title: tab.title.clone(), host: tab.host.clone() }).collect();
    tab_strip.active = browser.active;
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
/// `TabWebViews`/`TabStripState::collapsed` — the one place all three actually get mutated.
fn apply_browser_navigation(
    mut browser: ResMut<BrowserState>,
    mut webviews: NonSendMut<TabWebViews>,
    mut tab_strip: ResMut<TabStripState>,
    mut toolbar_events: MessageReader<ToolbarEvent>,
    mut tab_events: MessageReader<TabStripEvent>,
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
                tab_strip.collapsed = !tab_strip.collapsed;
                let width = tab_strip.effective_width();
                for webview in webviews.0.values() {
                    webview.set_left_inset(width);
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
        }
    }
}

fn open_tab(browser: &mut BrowserState, url: String) {
    let id = browser.next_id;
    browser.next_id += 1;
    let host = host_of(&url);
    browser.tabs.push(Tab { id, url: url.clone(), title: host.clone(), host, loading: false });
    browser.active = Some(id);
}

/// Resets `BrowserState` back to "no window open" the moment the browser window's entity stops
/// existing (the user closed it — Bevy despawns the entity itself, there's no separate "closed"
/// flag to read). Without this, `browser.window` keeps pointing at a dead `Entity` forever: every
/// later `/scene` call takes the "add a tab to the existing window" branch, `attach_pending_tab_
/// webviews`'s entity lookup fails silently, and nothing visibly happens — no error, no new
/// window, no tab. Must run before `spawn_scene_window_on_command` each tick so a `/scene` right
/// after a close creates a fresh window instead of one more silent no-op.
fn clear_browser_state_on_window_close(mut browser: ResMut<BrowserState>, mut webviews: NonSendMut<TabWebViews>, window_query: Query<Entity, With<bevy::window::Window>>) {
    let Some(window) = browser.window else { return };
    if window_query.get(window).is_err() {
        tracing::info!("Browser window closed; resetting tab state");
        *browser = BrowserState::default();
        webviews.0.clear();
    }
}

/// `/scene <url>` opens (or focuses) the single browser window instead of a brand new OS window
/// per call — the tabbed-browser redesign superseding the earlier one-window-per-scene approach.
/// First call creates the window (webview + toolbar + tab strip, `WindowLevel::AlwaysOnTop` only
/// in dev builds, same reasoning as before: easy to find while testing, not a real-use default);
/// every later call just opens a new tab in it.
fn spawn_scene_window_on_command(mut commands: Commands, mut scene_evt: MessageReader<SceneCommand>, mut browser: ResMut<BrowserState>, webviews: NonSend<TabWebViews>) {
    #[cfg(debug_assertions)]
    let window_level = bevy::window::WindowLevel::AlwaysOnTop;
    #[cfg(not(debug_assertions))]
    let window_level = bevy::window::WindowLevel::Normal;

    for SceneCommand { url } in scene_evt.read() {
        if browser.window.is_none() {
            let window_entity = commands
                .spawn((
                    escher_bevy::plugin::create_window("Anvil — Browser", 1100.0, 760.0, true, window_level),
                    WantsToolbar,
                    WantsTabStrip,
                ))
                .id();

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
        }
    }
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();

    color_eyre::install()?;

    // Every `tracing::*` call, from anywhere in the process, gets routed to a log file instead
    // of stdout — not just for this thread. A thread-local override (`with_default`, tried in
    // two earlier passes) can't reach everywhere a call might originate once persistence is in
    // the picture: libsql's async networking runs as its own tokio tasks, and its local
    // SQLite/WAL work runs on tokio's *blocking* thread pool — a separate pool that exists
    // regardless of runtime flavor. Both fall outside any thread-local scoping. Since the
    // terminal is in raw mode for nearly this whole run, any stray *printed* line corrupts the
    // screen — redirecting the global default itself is the only thing that covers every
    // thread uniformly. `RUST_LOG`/`--log-level` still controls verbosity; `tail -f
    // anvil.log` to watch it live.
    let log_file = std::fs::File::create("anvil.log")?;
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
    let transcript_layer = TranscriptLayer { sender: trace_tx };

    // Everything logged anywhere in the process — the same unscoped firehose `file_layer` above
    // writes to `anvil.log` — also lands here, live, in a bounded ring buffer. Backs
    // `Page::Trace` (toggled with F2 in `draw_assistant`): a way to see that raw feed without
    // leaving the app or tailing the log file in a second terminal. `.with_ansi(true)` is forced
    // rather than left to auto-detect, since `LineBuffer` as a `MakeWriter` isn't a real tty —
    // auto-detection would otherwise decide color should be off and strip it before it ever
    // reaches `LineBufferWriter`.
    let trace_buffer = LineBuffer::new();
    let trace_page_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_writer(trace_buffer.clone());

    // A raw subprocess stdio feed — `Page::Process`, toggled with F3 — fed directly by
    // `run_js_command`, not through `tracing` at all (see `LineBuffer`'s own doc comment for why
    // it's a separate buffer from `trace_buffer` above, not just another `tracing` layer).
    let process_buffer = LineBuffer::new();

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
        .with_writer(RawStreamGate { active: raw_stream_flag.clone() });

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
            let persistence = persistence::Persistence::connect().await?;
            persistence.reset().await?;
            println!("Cleared all persisted messages and tasks.");
            Ok(ExitCode::SUCCESS)
        });
    }

    let app_state =
        runtime.block_on(AppState::new(runtime.clone(), trace_rx, trace_buffer, process_buffer, raw_stream_flag.clone()));

    // `/scene` (the `Enter`-key handler further down) fires a real in-process `SceneCommand`
    // instead of spawning a second process for it — see `AssistantTerminalPlugin`'s own doc
    // comment for why that means this whole app is a Bevy app now. Every `SceneCommand` opens a
    // brand new, independent window (`spawn_scene_window_on_command`) — no single shared window to
    // pre-warm, so `spawn_primary_window(false)` means the app starts with none at all, until the
    // first `/scene`.
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
                .with_exit_condition(bevy::window::ExitCondition::DontExit),
        ))
        .add_plugins(OsPlugin::new("Anvil"))
        .add_plugins(ToolbarPlugin)
        .add_message::<SceneCommand>()
        .insert_non_send_resource(TabWebViews::default())
        .insert_resource(BrowserState::default())
        .insert_resource(TabStripState::default())
        .insert_resource(ThemeState(Some(ToolbarTheme {
            background: styleguide_color("background", (26, 27, 38)),
            surface: styleguide_color("surface", (31, 35, 53)),
            accent: styleguide_color("accent", (122, 162, 247)),
            text: styleguide_color("text", (192, 202, 245)),
            ui_text_size: styleguide_text_size("ui", 15.0),
            body_text_size: styleguide_text_size("body", 13.0),
        })))
        .insert_resource(app_state)
        .add_plugins(AssistantTerminalPlugin)
        .add_systems(
            Update,
            (clear_browser_state_on_window_close, spawn_scene_window_on_command, attach_pending_tab_webviews, sync_tab_loading_state, sync_toolbar_state)
                .chain()
                .before(ToolbarSystems),
        )
        .add_systems(Update, apply_browser_navigation.after(ToolbarSystems))
        .run();

    //--
    tracing::info!("Bye! <3");

    // By this point `assistant_terminal_exit` has already left the alternate screen and disabled
    // raw mode, so printing straight to stdout is safe again — it won't corrupt anything the app
    // was drawing, because it isn't drawing anymore.
    if args.dump_trace {
        match std::fs::read_to_string("anvil.log") {
            Ok(trace) => {
                println!("--- anvil.log ---");
                print!("{trace}");
            }
            Err(error) => {
                eprintln!("Failed to read anvil.log for --dump-trace: {error}");
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

/// One entry in the "running tasks" overlay.
#[derive(Debug, Clone)]
struct TaskRow {
    label: String,
    status: String, // "done" | "running" | "pending"
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
mod persistence {
    use std::time::Duration;

    use libsql::Builder;
    use libsql::Connection;
    use libsql::Database;
    use libsql::params;

    use super::ChatMessage;
    use super::TaskRow;

    /// Matches `tools/data/compose.yaml`'s `sqld` service (`8081:8080`).
    const SQLD_URL: &str = "http://localhost:8081";
    const LOCAL_REPLICA_PATH: &str = "anvil-replica.db";

    /// The connect/sync calls below have no timeout of their own, so a slow or unresponsive
    /// `sqld` would otherwise hang here indefinitely instead of failing into the documented
    /// in-memory fallback.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

    /// Within a single tool message's `output` lines, a real newline could legitimately appear
    /// in any one line — so joining/splitting on `\n` to store `Vec<String>` in one TEXT column
    /// would corrupt data with embedded newlines. `\u{1e}` (ASCII Record Separator) is exactly
    /// what it's for and won't collide with anything a task's fake output actually contains.
    const OUTPUT_SEPARATOR: char = '\u{1e}';

    pub struct Persistence {
        connection: Connection,
    }

    impl Persistence {
        /// Opens (creating if needed) the local replica, syncs once against `sqld`, and ensures
        /// the schema exists. Returns `Err` if `sqld` isn't reachable within `CONNECT_TIMEOUT`,
        /// including if it never responds at all. The caller falls back to in-memory-only
        /// operation rather than treating that as fatal, since this is a demo people will often
        /// run without `sqld` up.
        pub async fn connect() -> color_eyre::Result<Self> {
            match tokio::time::timeout(CONNECT_TIMEOUT, Self::connect_inner()).await {
                Ok(result) => result,
                Err(_) => Err(color_eyre::eyre::eyre!(
                    "Timed out connecting to sqld at {SQLD_URL} after {CONNECT_TIMEOUT:?}"
                )),
            }
        }

        async fn connect_inner() -> color_eyre::Result<Self> {
            let database: Database = Builder::new_remote_replica(
                LOCAL_REPLICA_PATH,
                SQLD_URL.to_string(),
                String::new(), // no auth configured on the local dev sqld instance
            )
            .build()
            .await?;

            database.sync().await?;

            let connection = database.connect()?;
            let persistence = Persistence { connection };
            persistence.ensure_schema().await?;
            Ok(persistence)
        }

        async fn ensure_schema(&self) -> color_eyre::Result<()> {
            self.connection
                .execute_batch(
                    "CREATE TABLE IF NOT EXISTS messages (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        kind TEXT NOT NULL,
                        content TEXT,
                        tool_name TEXT,
                        tool_detail TEXT,
                        tool_output TEXT,
                        created_at INTEGER NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS tasks (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        label TEXT NOT NULL,
                        status TEXT NOT NULL,
                        created_at INTEGER NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS overlay_state (
                        id INTEGER PRIMARY KEY CHECK (id = 1),
                        x INTEGER NOT NULL,
                        y INTEGER NOT NULL,
                        width INTEGER NOT NULL,
                        height INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL
                    );",
                )
                .await?;
            Ok(())
        }

        /// Wipes every message and task — used by `--reset-data`. Real deletes against the
        /// remote `sqld` primary (not just the local replica file), so this actually clears the
        /// data everyone connecting to this `sqld` instance sees, not just this machine's cache.
        pub async fn reset(&self) -> color_eyre::Result<()> {
            self.connection
                .execute_batch("DELETE FROM messages; DELETE FROM tasks; DELETE FROM overlay_state;")
                .await?;
            Ok(())
        }

        pub async fn load_messages(&self) -> color_eyre::Result<Vec<ChatMessage>> {
            let mut rows = self
                .connection
                .query(
                    "SELECT kind, content, tool_name, tool_detail, tool_output FROM messages ORDER BY id ASC",
                    (),
                )
                .await?;

            let mut messages = Vec::new();

            while let Some(row) = rows.next().await? {
                let kind: String = row.get(0)?;
                match kind.as_str() {
                    "user" => messages.push(ChatMessage::User(row.get(1)?)),
                    "assistant" => messages.push(ChatMessage::Assistant(row.get(1)?)),
                    "tool" => {
                        let output_joined: String = row.get::<Option<String>>(4)?.unwrap_or_default();
                        let output = if output_joined.is_empty() {
                            Vec::new()
                        } else {
                            output_joined.split(OUTPUT_SEPARATOR).map(String::from).collect()
                        };

                        messages.push(ChatMessage::Tool {
                            name: row.get(2)?,
                            detail: row.get(3)?,
                            output,
                        });
                    }
                    other => tracing::warn!("Unknown message kind {other:?} in database, skipping"),
                }
            }

            Ok(messages)
        }

        pub async fn save_message(&self, message: &ChatMessage) -> color_eyre::Result<()> {
            match message {
                ChatMessage::User(text) => {
                    self.connection
                        .execute(
                            "INSERT INTO messages (kind, content, created_at) VALUES ('user', ?1, ?2)",
                            params![text.as_str(), now_millis()],
                        )
                        .await?;
                }
                ChatMessage::Assistant(text) => {
                    self.connection
                        .execute(
                            "INSERT INTO messages (kind, content, created_at) VALUES ('assistant', ?1, ?2)",
                            params![text.as_str(), now_millis()],
                        )
                        .await?;
                }
                ChatMessage::Tool { name, detail, output } => {
                    let joined = output.join(&OUTPUT_SEPARATOR.to_string());
                    self.connection
                        .execute(
                            "INSERT INTO messages (kind, tool_name, tool_detail, tool_output, created_at) \
                             VALUES ('tool', ?1, ?2, ?3, ?4)",
                            params![name.as_str(), detail.as_str(), joined, now_millis()],
                        )
                        .await?;
                }
                // Ephemeral by design, see `ChatMessage::Trace`'s own doc comment.
                ChatMessage::Trace(_) => {}
            }
            Ok(())
        }

        pub async fn load_tasks(&self) -> color_eyre::Result<Vec<TaskRow>> {
            let mut rows = self
                .connection
                .query("SELECT label, status FROM tasks ORDER BY id ASC", ())
                .await?;

            let mut tasks = Vec::new();
            while let Some(row) = rows.next().await? {
                tasks.push(TaskRow { label: row.get(0)?, status: row.get(1)? });
            }
            Ok(tasks)
        }

        pub async fn save_task(&self, task: &TaskRow) -> color_eyre::Result<()> {
            self.connection
                .execute(
                    "INSERT INTO tasks (label, status, created_at) VALUES (?1, ?2, ?3)",
                    params![task.label.as_str(), task.status.as_str(), now_millis()],
                )
                .await?;
            Ok(())
        }

        /// `(x, y, width, height)` in plain `u16`s rather than `ratatui::layout::Rect` — this
        /// module deliberately doesn't depend on `ratatui` (see `ChatMessage`/`TaskRow`, defined
        /// outside it for the same reason), the caller reassembles the `Rect` itself.
        pub async fn load_overlay_bounds(&self) -> color_eyre::Result<Option<(u16, u16, u16, u16)>> {
            let mut rows = self.connection.query("SELECT x, y, width, height FROM overlay_state WHERE id = 1", ()).await?;

            match rows.next().await? {
                Some(row) => {
                    let x: i64 = row.get(0)?;
                    let y: i64 = row.get(1)?;
                    let width: i64 = row.get(2)?;
                    let height: i64 = row.get(3)?;
                    Ok(Some((x as u16, y as u16, width as u16, height as u16)))
                }
                None => Ok(None),
            }
        }

        /// A single-row upsert (`id` is `CHECK (id = 1)`-constrained to enforce that) — the
        /// overlay only ever has one position, there's nothing to key multiple rows on.
        pub async fn save_overlay_bounds(&self, bounds: (u16, u16, u16, u16)) -> color_eyre::Result<()> {
            let (x, y, width, height) = bounds;
            self.connection
                .execute(
                    "INSERT INTO overlay_state (id, x, y, width, height, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?5) \
                     ON CONFLICT (id) DO UPDATE SET x = ?1, y = ?2, width = ?3, height = ?4, updated_at = ?5",
                    params![x as i64, y as i64, width as i64, height as i64, now_millis()],
                )
                .await?;
            Ok(())
        }
    }

    fn now_millis() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }
}

// Escher doesn't yet support sizing a child slot to its own wrapped content from the parent's
// layout pass — every slot without an explicit `Size` gets an equal share of whatever space is
// left, not a content-fitted one. So instead of one slot per message (which left uneven dead
// space under short turns), the whole transcript is rendered as a single `Body` content block,
// and `Overflow::Scroll` + `ScrollPosition` (see `draw_assistant`) show a window into it —
// following the bottom by default, or pinned wherever the user scrolled to with PageUp/PageDown.
const HEADER_HEIGHT: u16 = 1;
const FOOTER_HEIGHT: u16 = 3;
const STATUS_HEIGHT: u16 = 1;
/// A blank row between the transcript (or the autocomplete bar, when it's showing) and the
/// input box below it — without this the input's own border sat directly against whatever was
/// above it with no breathing room at all.
const INPUT_GAP_HEIGHT: u16 = 1;

/// How long the overlay's position has to sit unchanged before `sync_overlay_bounds_to_
/// persistence` writes it to sqld — long enough that a `Drag` in progress (many events a
/// second while the mouse is moving) never triggers a write per event, short enough that
/// letting go still saves promptly rather than needing a deliberately idle pause afterward.
const OVERLAY_PERSIST_DEBOUNCE: Duration = Duration::from_millis(400);

/// How long a copy-selection problem shows a short warning before the status line escalates to
/// the fuller hint about the terminal's mouse-override shortcut.
const MOUSE_HINT_DELAY: Duration = Duration::from_secs(6);
/// Past this age with no further failed attempts, the hint is treated as stale and stops
/// showing even though `mouse_trouble_since` hasn't been explicitly cleared, so a one-off
/// failed copy doesn't leave a permanent warning for a user who's moved on.
const MOUSE_HINT_MAX_AGE: Duration = Duration::from_secs(30);

/// Shared token source for this app's terminal UI *and* its native AppKit toolbar/tab strip (see
/// `ThemeState` in `escher_appkit::bevy`, populated from this same instance in `main()`) — one
/// palette read by both surfaces instead of each hardcoding its own. See `escher-styleguide` for
/// the parser and `anvil.styleguide.md` for the actual token values.
static STYLEGUIDE: LazyLock<escher_styleguide::Styleguide> =
    LazyLock::new(|| escher_styleguide::Styleguide::parse(include_str!("../anvil.styleguide.md")).expect("anvil.styleguide.md must parse"));

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
// value comes from `anvil.styleguide.md`, with the original hardcoded value kept as a fallback in
// case that file is ever missing a token (it shouldn't be, in the normal build).
static ACCENT_BLUE: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("accent", (122, 162, 247)));
static ACCENT_ORANGE: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("accent-warn", (224, 175, 104)));
static GREEN: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("success", (158, 230, 106)));
static RED: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("danger", (247, 118, 142)));
static DIM: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("text-muted", (86, 95, 137)));
static BACKGROUND: LazyLock<(u8, u8, u8)> = LazyLock::new(|| styleguide_color("background", (26, 27, 38)));

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
    args_hint: String,
    description: String,
    script: Option<PathBuf>,
}

fn builtin_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand { name: "task".into(), args_hint: "<label>".into(), description: "Add a task to the overlay".into(), script: None },
        SlashCommand { name: "scene".into(), args_hint: "<url>".into(), description: "Open a Bevy scene with a webview".into(), script: None },
    ]
}

/// Every `.js` file in `dir` becomes a `/<filename>` command, args-hint `<args>` (a script can't
/// declare a more specific one — it's just whatever text follows the command name) and
/// description naming the script so it's clear in the autocomplete list where it came from.
/// Missing/unreadable `dir` just means no JS commands, not a startup failure — this is example
/// content, not something the app depends on to run.
///
/// `clear` is hardcoded as an exception with an empty args-hint (see `autocomplete_bar_text`
/// for how that renders as a bare `/clear`) since it's the only script today that genuinely takes
/// no arguments — not worth a general per-script args-declaration mechanism for one case.
fn discover_js_commands(dir: &Path) -> Vec<SlashCommand> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut commands: Vec<SlashCommand> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "js"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_owned();
            let args_hint = if name == "clear" { "".into() } else { "<args>".into() };
            Some(SlashCommand {
                description: format!("Run {} (commands/{}.js)", name, name),
                name,
                args_hint,
                script: Some(path),
            })
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

/// Every known command whose name starts with `partial_name` — `""` matches everything (bare
/// `/` shows the full list), a full command name still matches itself (so Tab/Enter can accept
/// an exact, unambiguous match too, not just narrow a still-ambiguous prefix). Clones rather
/// than borrows — `commands` is recomputed fresh each frame and these results get captured into
/// a `move` closure below; a handful of small string clones a frame isn't worth threading
/// lifetimes through several closures to avoid.
fn matching_commands(commands: &[SlashCommand], partial_name: &str) -> Vec<SlashCommand> {
    commands.iter().filter(|command| command.name.starts_with(partial_name)).cloned().collect()
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

/// Runs a JS command script via `ethos-cli`'s `run-command` (see `ethos/apps/cli/src/main.rs`)
/// as a child process. Kept a separate process rather than linking `ethos-deno` directly into
/// this example, since the two runtimes (V8/Deno here, `escher-core`'s bump-arena UI tree) are
/// independently large and this keeps them decoupled. `ensure_ethos_cli_built` runs `cargo build`
/// (not `cargo run`) first, so a source change to `ethos-cli` or anything it depends on always
/// gets picked up before the resulting binary is invoked directly. `cargo build`'s own freshness
/// check keeps that cheap in the common case, and only actually slow (well over a minute) right
/// after something genuinely changed, same as `cargo run` always was. The caller (the `Enter`-key
/// handler) runs this whole function on a background thread via `AppState::spawn_js_command`, not
/// inline. That slow case, and even an ordinary fork-and-wait for a fast case, would otherwise
/// visibly freeze the UI for however long it takes.
/// Runs `ethos-cli run-command`, forwarding every line of its stdout/stderr live via
/// `tracing::*` as they're produced instead of only returning them once the whole process
/// exits — `ethos-cli` sets up its own `tracing_subscriber::fmt()` writing to stdout (see
/// `apps/cli/src/main.rs`), and a `.js`/`.ts` script's own `console.log` calls land there too
/// (documented in `ethos-cli`'s `run-command` help text), so this is the one place both "the
/// ffi-bound scripting layer's tracing" and "a script reporting its own progress" already meet,
/// no new protocol needed on the script side. Forwarding only works because this function always
/// runs inside a `live_trace` span entered by its caller (`AppState::spawn_js_command`) — see
/// `TranscriptLayer`. The final return value is unchanged from before this streaming was added:
/// stdout joined back together and trimmed on success, stderr on failure.
fn run_js_command(script: &Path, args: &str, command_label: &str, process_buffer: &LineBuffer) -> Result<String, String> {
    let ethos_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../ethos");
    let binary = ensure_ethos_cli_built(&ethos_root)?;

    let mut child = Command::new(binary)
        .arg("run-command")
        .arg(script)
        .arg(args)
        .current_dir(&ethos_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn ethos-cli: {error}"))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    // A header line per run (dim, not part of the child's own output) so `Page::Process`'s
    // continuous scrollback across every command this session stays legible about where one
    // run ends and the next begins — the raw stdio itself carries no such marker on its own.
    process_buffer.push_line(format!("{}", format!("── {command_label} ──").truecolor(DIM.0, DIM.1, DIM.2)));

    // Stderr drains on its own thread so a child that writes a lot to one stream while this
    // thread blocks reading the other can't deadlock both of them on a full pipe buffer. Each
    // line reaches three places independently: `tracing::warn!` (the existing `live_trace`-
    // scoped forwarding into the chat transcript, unchanged), and `process_buffer` (new — the
    // exact raw line, no `tracing` formatting/timestamp/level prefix at all, pushed directly
    // from both threads as lines actually arrive, so `Page::Process`'s stdout/stderr
    // interleaving tracks real arrival order the same way a real terminal's combined stream
    // would, not just concatenated after the fact).
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

    let status = child.wait().map_err(|error| format!("failed waiting for ethos-cli: {error}"))?;

    if !status.success() {
        return Err(stderr_lines.join("\n").trim().to_string());
    }

    Ok(stdout_lines.join("\n").trim_end().to_string())
}

/// Runs `cargo build` for `ethos-cli`'s `ethos` binary, every call, not just when the binary is
/// missing, so a source change (to `ethos-cli` or anything it depends on) always gets picked up
/// rather than silently running a stale build. `cargo build`'s own freshness check makes this
/// cheap when nothing changed (the common case), and only actually slow the same way `cargo run`
/// was when something really did change. That cost now happens off the render thread (see
/// `AppState::spawn_js_command`), so it no longer matters that it's slow. What this still saves
/// over `cargo run`: the extra process-wrapping `cargo run` does to exec the target binary as a
/// child of itself, on top of the same freshness check either way.
fn ensure_ethos_cli_built(ethos_root: &Path) -> Result<PathBuf, String> {
    // `.output()`, not `.status()` — `status()` inherits the parent's stdout/stderr, and cargo
    // still prints compiler errors and warnings even with `--quiet`. Inheriting that output would
    // write raw text straight into this app's raw-mode alternate screen from a background thread,
    // corrupting the display, the same class of bug `spawn_bevy_scene` avoids by redirecting its
    // own child's stdio instead of inheriting it.
    let output = Command::new("cargo")
        .args(["build", "-p", "ethos-cli", "--bin", "ethos", "--quiet"])
        .current_dir(ethos_root)
        .output()
        .map_err(|error| format!("failed to build ethos-cli: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(ethos_root.join(".cargo/target/debug/ethos"))
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
}

#[derive(Clone, Resource)]
struct AppState {
    messages: Arc<RwLock<Vec<ChatMessage>>>,
    tasks: Arc<RwLock<Vec<TaskRow>>>,
    user_input: Arc<RwLock<String>>,
    /// The full command list — `task`/`scene` plus whatever `.js` files
    /// `commands/` held at startup. Fixed for the app's lifetime (no live-reload of
    /// the commands directory), so a plain `Vec` rather than behind an `Arc<RwLock<_>>` like
    /// the rest of this struct's mutable state.
    commands: Vec<SlashCommand>,
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
    /// Just a clock for animating the fake "running task" spinner in the overlay — not real work.
    start: Instant,
    /// `None` until the background connect task in `AppState::new` finishes, and stays `None`
    /// permanently if `sqld` wasn't reachable within `persistence::CONNECT_TIMEOUT` (see the
    /// `persistence` module). The app runs fully either way, just without persistence, rather
    /// than treating that as fatal or blocking startup on it — a demo shouldn't stall or refuse
    /// to start because a docker-compose service isn't up.
    persistence: Arc<RwLock<Option<Arc<persistence::Persistence>>>>,
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
    /// The overlay's position as of the last frame, and when it last changed — drives the
    /// debounce in `draw_assistant` that decides a move/resize gesture has actually settled and
    /// is worth writing to sqld, rather than saving on every single `Drag` event mid-gesture.
    overlay_bounds_seen: Arc<RwLock<Option<Rect>>>,
    overlay_bounds_changed_at: Arc<RwLock<Option<Instant>>>,
    /// `/scene <url>` commands, queued here instead of acted on directly — the `CrosstermEvent`
    /// handler that catches Enter runs inside `TerminalSurface::draw`'s own dispatch, not as a
    /// Bevy system, so it can't take a `MessageWriter<SceneCommand>` directly. Drained by
    /// `AssistantTerminalPlugin::draw_ui` (see `main`) into real `SceneCommand` writes once it's
    /// back in normal system context — the same workaround `escher_bevy::terminal::
    /// TerminalProvider::pending_scenes` already uses for the same reason.
    pending_scenes: Arc<Mutex<Vec<String>>>,
    /// Set (from `draw_assistant`'s F1 handler, or `--no-tui`'s startup value) to leave the TUI
    /// for the plain raw trace stream `RawStreamGate` prints straight to stdout, cleared (from
    /// `assistant_terminal_draw`'s own raw poll loop, once that mode is active) to return to the
    /// TUI — see `RawStreamGate`'s own doc comment for why this exists as a plain `AtomicBool`
    /// shared with a tracing writer, rather than the `Arc<RwLock<_>>` the rest of this struct
    /// uses for shared state.
    raw_stream: Arc<AtomicBool>,
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
    ) -> Self {
        let mut commands = builtin_commands();
        commands.extend(discover_js_commands(&Path::new(env!("CARGO_MANIFEST_DIR")).join("commands")));

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
            trace_buffer,
            process_scroll: Arc::new(RwLock::new(ScrollState::default())),
            process_buffer,
            start: Instant::now(),
            persistence: Arc::new(RwLock::new(None)),
            runtime,
            frame_times: Arc::new(RwLock::new(VecDeque::new())),
            mouse_trouble_since: Arc::new(RwLock::new(None)),
            trace_rx: Arc::new(Mutex::new(trace_rx)),
            persisted_overlay_bounds: Arc::new(RwLock::new(None)),
            overlay_bounds_seen: Arc::new(RwLock::new(None)),
            overlay_bounds_changed_at: Arc::new(RwLock::new(None)),
            pending_scenes: Arc::new(Mutex::new(Vec::new())),
            raw_stream,
        };

        state.spawn_connect_persistence();
        state
    }

    /// Connects to `sqld` and loads existing messages/tasks on `self.runtime`, without blocking
    /// the caller — the TUI is already interactive by the time this finishes or times out.
    /// `messages`/`tasks` jump from empty to populated once it succeeds, which is a better
    /// experience than staring at a blank terminal for up to `persistence::CONNECT_TIMEOUT`
    /// before anything appears at all.
    fn spawn_connect_persistence(&self) {
        let persistence = self.persistence.clone();
        let messages = self.messages.clone();
        let tasks = self.tasks.clone();
        let persisted_overlay_bounds = self.persisted_overlay_bounds.clone();

        self.runtime.spawn(async move {
            let store = match persistence::Persistence::connect().await {
                Ok(store) => store,
                Err(error) => {
                    tracing::warn!("Could not connect to sqld — running without persistence: {error}");
                    return;
                }
            };

            match store.load_messages().await {
                Ok(loaded) => *messages.write() = loaded,
                Err(error) => tracing::warn!("Failed to load messages from sqld, starting empty: {error}"),
            }

            match store.load_tasks().await {
                Ok(loaded) => *tasks.write() = loaded,
                Err(error) => tracing::warn!("Failed to load tasks from sqld, starting empty: {error}"),
            }

            match store.load_overlay_bounds().await {
                Ok(loaded) => *persisted_overlay_bounds.write() = loaded,
                Err(error) => tracing::warn!("Failed to load overlay position from sqld, using the default: {error}"),
            }

            *persistence.write() = Some(Arc::new(store));
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
                    run_js_command(&script, &args, &command_label, &process_buffer)
                })
                .await
                .unwrap_or_else(|error| Err(format!("js command task panicked: {error}")))
            };

            let reply = match result {
                Ok(output) if !output.is_empty() => output,
                Ok(_) => "(script ran, but returned nothing)".to_string(),
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

    /// Content for the floating "running tasks" overlay — real tasks only (`/task <label>` in
    /// the input; this is still a UI demo, not a real task runner, so new tasks just sit at
    /// "pending" forever), the spinner glyph is the only genuinely animated part, from
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

        // Doubles as the answer to "how do I add tasks"/"how do I open a scene"/"how do I look
        // at a task" — nowhere else in the UI says any of it.
        lines.push(format!("{}", "/task <label>".truecolor(DIM.0, DIM.1, DIM.2)));
        lines.push(format!("{}", "/scene <url>".truecolor(DIM.0, DIM.1, DIM.2)));
        if !self.tasks.read().is_empty() {
            lines.push(format!("{}", "↑↓ select a task".truecolor(DIM.0, DIM.1, DIM.2)));
        }

        // Breathing room comes from the overlay's own `Padding::left(1)`/`Padding::right(1)`
        // style now, not a hand-rolled leading blank line + per-line indent — the overlay-height
        // calculation at this function's call site has to stay in sync with the line count here:
        // 1 title + one per task + 2 or 3 hint lines.
        lines.join("\n")
    }
}

/// Content for the dedicated `AutocompleteBar` slot rendered directly above the input while a
/// `/command` name is being typed — one row per match with the currently-selected one picked out
/// in accent/bold (Up/Down moves `selected_index`), plus a keybinding hint. No title line or
/// border chrome the way the tasks overlay's content needs — this is an inline bar sitting right
/// above the input it's completing, not a standalone floating box, so it doesn't need to caption
/// or frame itself the same way. Line count has to
/// stay in sync with `draw_assistant`'s `autocomplete_bar_height` calculation: one row per match
/// + 1 hint line.
fn autocomplete_bar_text(matches: &[SlashCommand], selected_index: usize) -> String {
    let mut lines = Vec::with_capacity(matches.len() + 1);

    for (i, command) in matches.iter().enumerate() {
        let label = if command.args_hint.is_empty() {
            format!("/{}", command.name)
        } else {
            format!("/{} {}", command.name, command.args_hint)
        };
        let entry = format!("{}  {}", label, command.description);

        let line = if i == selected_index {
            format!("{} {}", "▸".truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2), entry.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold())
        } else {
            format!("  {}", entry.truecolor(DIM.0, DIM.1, DIM.2))
        };

        lines.push(line);
    }

    lines.push(format!("{}", "↑↓ navigate · Tab/Enter accept".truecolor(DIM.0, DIM.1, DIM.2)));

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
        "{}\n\n{} {}\n\n{}",
        task.label.as_str().truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold(),
        format!("{}", "Status:".truecolor(DIM.0, DIM.1, DIM.2)),
        status_label,
        "No linked activity yet — nothing in this app connects a task back to specific messages \
         or tool calls yet.".truecolor(DIM.0, DIM.1, DIM.2),
    )
}

/// Word-wraps `text` to `width` columns, with `gutter` prepended to the first line and blank
/// padding of the same display width prepended to every line after — i.e. a hanging indent.
fn wrap_hanging(text: &str, gutter: &str, width: usize) -> String {
    let indent_width = UnicodeWidthStr::width(gutter);
    let content_width = width.saturating_sub(indent_width).max(1);

    wrap_words(text, content_width)
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                format!("{}{}", gutter, line)
            } else {
                format!("{}{}", " ".repeat(indent_width), line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Greedy word-wrap at `width` display columns — breaks at whitespace where possible, and
/// hard-breaks any single token wider than `width` on its own (mirrors what `Paragraph::wrap`
/// does for unbreakable tokens, e.g. a long path with no spaces). Doesn't add any indentation
/// itself; see `wrap_hanging` for that.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0usize;

        for word in paragraph.split(' ') {
            let word_width = UnicodeWidthStr::width(word);

            if word_width > width {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                }
                lines.extend(hard_break(word, width));
                continue;
            }

            if current.is_empty() {
                current.push_str(word);
                current_width = word_width;
            } else if current_width + 1 + word_width <= width {
                current.push(' ');
                current.push_str(word);
                current_width += 1 + word_width;
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
                current_width = word_width;
            }
        }

        lines.push(current);
    }

    lines
}

/// Splits a single unbreakable token into `width`-wide chunks, for the case a word alone is
/// too long to fit any line no matter what (e.g. a long path, or a long unbroken test string).
fn hard_break(word: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for c in word.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > width && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(c);
        current_width += char_width;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

/// A smooth 0..1 "breathing" value from a sine wave over `period`, rather than something that
/// snaps between fixed states — reads as a calm, quietly-alive indicator instead of a blink.
fn breathe(elapsed: Duration, period: Duration) -> f64 {
    let phase = (elapsed.as_millis() % period.as_millis().max(1)) as f64 / period.as_millis() as f64;
    (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let lerp = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    (lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2))
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

    let Some(store) = state.persistence.read().clone() else { return };

    state.runtime.block_on(async {
        if let Err(error) = store.save_overlay_bounds(bounds).await {
            tracing::warn!("Failed to persist overlay position to sqld: {error}");
            return;
        }
        *state.persisted_overlay_bounds.write() = Some(bounds);
    });
}

fn draw_assistant(
    surface: &mut TerminalSurface<CrosstermBackend<Stdout>>,
    state: &AppState,
) -> Result<TerminalAction> {
    // The terminal size is already known before the scaffold tree is built (unlike the Body
    // slot's own rect, which only exists after layout), so the transcript's total wrapped
    // height — needed to know how far it *can* scroll — has to be computed out here rather
    // than inside the `with_slot::<Body>` closure below.
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
    let autocomplete_selected_index = if is_autocompleting {
        (*state.autocomplete_index.read()).min(autocomplete_matches.len() - 1)
    } else {
        0
    };

    // 0 rows (hidden) when not autocompleting; otherwise one row per match + 1 hint line, same
    // count `autocomplete_bar_text` actually renders.
    let autocomplete_bar_height = if is_autocompleting { autocomplete_matches.len() as u16 + 1 } else { 0 };

    let body_height =
        area.height.saturating_sub(HEADER_HEIGHT + autocomplete_bar_height + INPUT_GAP_HEIGHT + FOOTER_HEIGHT + STATUS_HEIGHT);
    // Body gets a 1-column pad on each side (below) so its text lines up with the Footer's
    // input, which is 1 column in from its own border — so text has to wrap 2 columns
    // narrower than the raw terminal width to end up in the same place at render time.
    let body_width = area.width.saturating_sub(2) as usize;

    let input_display = highlight_slash_command(&state.commands, &state.user_input.read());

    // A slow, subtle breathe on the overlay's border — dips partway toward dim and back over a
    // few seconds, not a hard blink, so it reads as "quietly still alive" rather than demanding
    // attention. Kept to a fairly small range (`* 0.4`) so it stays clearly the accent color.
    let overlay_pulse = lerp_color(*ACCENT_BLUE, *DIM, breathe(state.start.elapsed(), Duration::from_millis(2600)) * 0.4);

    // The overlay always shows the tasks list now — command suggestions moved to their own
    // `AutocompleteBar` slot above the input (see `autocomplete_bar_height`/
    // `autocomplete_bar_text`), so this slot no longer needs to swap content with autocomplete
    // (`Scaffold` only supports one detached overlay at a time — see `with_overlay`'s doc
    // comment in `escher_core::scaffold` — but there's nothing left to share it with here).
    // Interior rows: 1 title + one row per task + 2 hint lines, +1 more once there's at least
    // one task to select ("↑↓ select a task" only makes sense once selecting one is possible).
    // Plus 2 for the border — no extra rows for padding, since the overlay's
    // `Padding::left(1)`/`Padding::right(1)` is horizontal-only.
    let task_count = state.tasks.read().len() as u16;
    let overlay_height = task_count + if task_count > 0 { 6 } else { 5 };

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
        Page::Chat => match &selected_task {
            Some(task) => task_detail_text(task),
            None => {
                let messages = state.messages.read();
                build_transcript(&messages, expanded, body_width)
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
        Page::Chat => &state.scroll,
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
        Page::Chat => selected_task.is_none(),
    };
    let body_content = if pads_to_bottom && natural_offset == 0 {
        let padding_rows = body_height.saturating_sub(content_height);
        "\n".repeat(padding_rows as usize) + &body_content
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
        let turn_count = state.messages.read().len();
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "?".into());

        terminal_root
            .with_handler::<CrosstermEvent>({
                let user_input = state.user_input.clone();
                let autocomplete_index = state.autocomplete_index.clone();
                let messages = state.messages.clone();
                let tasks = state.tasks.clone();
                let expanded_flag = state.expanded.clone();
                let scroll = state.scroll.clone();
                let trace_scroll = state.trace_scroll.clone();
                let process_scroll = state.process_scroll.clone();
                let page = state.page.clone();
                let persistence = state.persistence.clone();
                let runtime = state.runtime.clone();
                let commands = state.commands.clone();
                let selected_task = state.selected_task.clone();
                let pending_scenes = state.pending_scenes.clone();
                let raw_stream = state.raw_stream.clone();
                let state_for_js = state.clone();
                let autocomplete_matches = autocomplete_matches.clone();
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
                    Page::Chat => &scroll,
                };
                match event {
                    CrosstermEvent::Key(key) => match key.code {
                        // A Ctrl-held character is a shortcut (Ctrl+C to copy a selection —
                        // handled in `TerminalSurface::draw` — plus whatever else in this
                        // modifier space), never literal text; typing it here too would insert
                        // a stray "c" into the input on every copy.
                        KeyCode::Char(key_char) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if key.kind != KeyEventKind::Release {
                                user_input.write().push(key_char);
                                *autocomplete_index.write() = 0;
                            }
                        }
                        KeyCode::Backspace => {
                            if key.kind != KeyEventKind::Release {
                                user_input.write().pop();
                                *autocomplete_index.write() = 0;
                            }
                        }
                        // While autocompleting, Up/Down navigate the `AutocompleteBar` suggestion
                        // list. Otherwise they navigate the task list instead, swapping the Body
                        // area to that task's own page — see the second match arm below. The two
                        // can't both apply at once (a `/` with no space yet is never a valid
                        // moment to also be browsing tasks), so there's no ambiguity about which
                        // Up/Down means.
                        KeyCode::Up if is_autocompleting => {
                            if key.kind != KeyEventKind::Release {
                                let mut index = autocomplete_index.write();
                                *index = index.checked_sub(1).unwrap_or(autocomplete_matches.len() - 1);
                            }
                        }
                        KeyCode::Down if is_autocompleting => {
                            if key.kind != KeyEventKind::Release {
                                let mut index = autocomplete_index.write();
                                *index = (*index + 1) % autocomplete_matches.len();
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
                        // Accepts the highlighted suggestion while autocompleting (completes the
                        // input to "/name ", ready for args); otherwise Tab keeps its existing
                        // job of toggling tool-call detail.
                        KeyCode::Tab => {
                            if key.kind != KeyEventKind::Release {
                                if is_autocompleting {
                                    if let Some(command) = autocomplete_matches.get(autocomplete_selected_index) {
                                        *user_input.write() = format!("/{} ", command.name);
                                        *autocomplete_index.write() = 0;
                                    }
                                } else {
                                    let mut expanded_flag = expanded_flag.write();
                                    *expanded_flag = !*expanded_flag;
                                }
                            }
                        }
                        // Leaves the TUI entirely for `RawStreamGate`'s plain trace stream — a
                        // different axis from F2/F3 below (those swap *within* the TUI's own Body
                        // area; this leaves the TUI, `Scaffold`/`TerminalSurface` and all). Only
                        // ever turns it on from here; turning it back off happens in
                        // `assistant_terminal_draw`'s own raw, non-`Scaffold` poll loop
                        // (`run_raw_stream_tick`) once that mode is actually active — the same
                        // "own its own input while active" split `spawn_input_watcher` uses
                        // elsewhere in this file.
                        KeyCode::F(1) => {
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
                        KeyCode::Enter if is_autocompleting => {
                            // Same as Tab while autocompleting — accepts the suggestion instead
                            // of submitting. A still-ambiguous or not-yet-typed command name
                            // shouldn't submit as free text just because Enter was pressed.
                            if key.kind != KeyEventKind::Release
                                && let Some(command) = autocomplete_matches.get(autocomplete_selected_index)
                            {
                                *user_input.write() = format!("/{} ", command.name);
                                *autocomplete_index.write() = 0;
                            }
                        }
                        KeyCode::Enter => {
                            if key.kind != KeyEventKind::Release {
                                let mut user_input = user_input.write();
                                let prompt = user_input.trim().to_owned();
                                user_input.clear();
                                drop(user_input);

                                if let Some(label) = prompt.strip_prefix("/task ").map(str::trim).filter(|l| !l.is_empty()) {
                                    let new_task = TaskRow { label: label.to_owned(), status: "pending".into() };

                                    if let Some(store) = persistence.read().as_ref() {
                                        runtime.block_on(async {
                                            if let Err(error) = store.save_task(&new_task).await {
                                                tracing::warn!("Failed to persist task to sqld: {error}");
                                            }
                                        });
                                    }

                                    tasks.write().push(new_task);
                                } else if let Some(url) = prompt.strip_prefix("/scene ").map(str::trim).filter(|u| !u.is_empty()) {
                                    // This whole app now runs *as* a Bevy app (see
                                    // `AssistantTerminalPlugin` in `main`) instead of owning its
                                    // own event loop, specifically so this can be a real in-
                                    // process `SceneCommand` instead of spawning a second `cargo
                                    // run` process and paying its full build-freshness-check and
                                    // cold-start cost on every single `/scene` call. Queued here
                                    // rather than written directly — this handler runs inside
                                    // `TerminalSurface::draw`'s own dispatch, not as a Bevy
                                    // system, so it can't take a `MessageWriter` — and drained by
                                    // `AssistantTerminalPlugin::draw_ui` once it's back in normal
                                    // system context (see `AppState::pending_scenes`).
                                    pending_scenes.lock().push(url.to_string());

                                    messages.write().push(ChatMessage::Assistant(format!("Opening a scene with a webview loaded to {url} …")));
                                } else if let Some((name, args)) = parse_slash_command(&prompt)
                                    && let Some(command) = commands.iter().find(|command| command.name == name)
                                    && let Some(script) = &command.script
                                {
                                    let user_message = ChatMessage::User(prompt.clone());

                                    if let Some(store) = persistence.read().as_ref() {
                                        runtime.block_on(async {
                                            if let Err(error) = store.save_message(&user_message).await {
                                                tracing::warn!("Failed to persist message to sqld: {error}");
                                            }
                                        });
                                    }

                                    messages.write().push(user_message);

                                    // Runs on a background thread, see `AppState::
                                    // spawn_js_command`'s doc comment for why this can't block
                                    // the render thread here. The reply appears a moment later,
                                    // once the script actually finishes.
                                    let command_name = format!("{} {}", command.name, args).trim().to_owned();
                                    state_for_js.spawn_js_command(command_name, script.clone(), args.to_owned());
                                } else if !prompt.is_empty() {
                                    // No fabricated `Bash echo`/"Got it — N characters" reply
                                    // anymore — that was leftover scripted-demo theater from
                                    // before real data was the whole point. There's no real
                                    // agent wired up yet (a real Eden/Atlas-backed one is coming
                                    // soon) — recording exactly what was actually typed, with no
                                    // invented response pretending otherwise, is the honest thing
                                    // to show in the meantime.
                                    let new_messages = [ChatMessage::User(prompt.clone())];

                                    if let Some(store) = persistence.read().as_ref() {
                                        // Blocking briefly here (a local sqld round-trip, not a
                                        // real network) is a deliberate, documented trade-off —
                                        // see the `persistence` module doc comment.
                                        runtime.block_on(async {
                                            for message in &new_messages {
                                                if let Err(error) = store.save_message(message).await {
                                                    tracing::warn!("Failed to persist message to sqld: {error}");
                                                }
                                            }
                                        });
                                    }

                                    messages.write().extend(new_messages);
                                }

                                // Sending a message always jumps back to the bottom, like any
                                // chat app — and, since a task might have been selected (Body
                                // showing its page instead of the transcript), back to the
                                // transcript too, so the message you just sent is visible.
                                *scroll.write() = ScrollState::Following;
                                *selected_task.write() = None;
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
            // A single double-line rule spanning the full width, with the title centered directly
            // in it — one terminal row, not the two-row iTerm2-style banner this used to be.
            .with_slot::<Header>(|header| {
                let title = format!(" {} ", "ESCHER TERMINAL ASSISTANT");
                let title_width = UnicodeWidthStr::width(title.as_str());
                let total_width = area.width as usize;
                let left_width = total_width.saturating_sub(title_width) / 2;
                let right_width = total_width.saturating_sub(left_width + title_width);
                let left_bar = "═".repeat(left_width);
                let right_bar = "═".repeat(right_width);

                header
                    .with_style(Size::height(HEADER_HEIGHT))
                    .with_content(Some(format!(
                        "{}{}{}",
                        left_bar.as_str().truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold(),
                        title.truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold(),
                        right_bar.as_str().truecolor(ACCENT_BLUE.0, ACCENT_BLUE.1, ACCENT_BLUE.2).bold(),
                    )))
            })
            .with_slot::<Body>(|body| {
                body
                    // Lines up the transcript's left edge with the Footer's input, which sits
                    // 1 column in from its own border.
                    .with_style(Padding::left(1))
                    .with_style(Padding::right(1))
                    .with_style(Overflow::Scroll)
                    .with_style(ScrollPosition::new(scroll_offset))
                    .with_content(Some(body_content))
            })
            // A small dynamic bar showing `/command` matches while one's being typed, sitting
            // directly above the input it's completing — easier to spot right where you're
            // typing than the old shared-overlay dropdown in the top-right corner. Zero height
            // (and no content) when not autocompleting, so it takes up no space the rest of the
            // time; `autocomplete_bar_height` (computed above, alongside `body_height`) has to
            // stay in sync with the row count `autocomplete_bar_text` actually renders.
            .with_slot::<AutocompleteBar>(|bar| {
                bar.with_style(Size::height(autocomplete_bar_height))
                    .with_style(Padding::left(1))
                    .with_style(Padding::right(1))
                    .with_content(is_autocompleting.then(|| autocomplete_bar_text(&autocomplete_matches, autocomplete_selected_index)))
            })
            .with_slot::<InputGap>(|gap| gap.with_style(Size::height(INPUT_GAP_HEIGHT)))
            .with_slot::<Footer>(|footer| {
                // A steady 530ms on/off cadence — the common terminal-emulator cursor blink
                // rate (iTerm2/Terminal.app's default) — rather than the smooth `breathe()`
                // pulse used elsewhere; a text-input caret reads as "blinking", not "glowing".
                let cursor_visible = (state.start.elapsed().as_millis() / 530) % 2 == 0;

                footer
                    .with_style(FlexDirection::Row)
                    .with_style(Size::height(FOOTER_HEIGHT))
                    .with_style(Border::new(1, BorderStyle::Solid, Some(Color::new(ACCENT_ORANGE.0, ACCENT_ORANGE.1, ACCENT_ORANGE.2, 255))))
                    .with_element(Input::<String>::new(input_display.clone()).with_cursor_visible(cursor_visible))
            })
            // Model/turn/keybinding info lives below the input now — easier to spot right next
            // to where you're actually typing than tucked into the header above the transcript.
            .with_slot::<StatusLine>(|status| {
                let scroll_hint = if is_scrolled_up {
                    format!(" · {}", "↑ scrolled (PgDn to catch up)".truecolor(ACCENT_ORANGE.0, ACCENT_ORANGE.1, ACCENT_ORANGE.2))
                } else {
                    String::new()
                };

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

                status
                    .with_style(Size::height(STATUS_HEIGHT))
                    .with_style(Padding::left(1))
                    .with_style(FontStyle::Italic)
                    .with_style(ContentColor::new(DIM.0, DIM.1, DIM.2, 255))
                    .with_content(Some(match mouse_hint {
                        Some(hint) => format!("{}", hint.truecolor(RED.0, RED.1, RED.2)),
                        None => format!(
                            "demo-1 · {} turns · {} · {} fps · Tab: {} tools · F1: raw stream · F2: {} trace · F3: {} process · PgUp/PgDn: scroll{}",
                            turn_count,
                            cwd,
                            fps,
                            if expanded { "hide" } else { "show" },
                            if page == Page::Trace { "hide" } else { "show" },
                            if page == Page::Process { "hide" } else { "show" },
                            scroll_hint,
                        ),
                    }))
            })
            // A floating window layered over the transcript instead of taking up its own row —
            // `with_overlay` renders a detached scaffold at a fixed corner (see
            // `TerminalSurface::overlay_rect`) rather than partitioning space like a slot does.
            .with_overlay(|overlay| {
                overlay
                    .with_style(Size(OVERLAY_WIDTH.into(), overlay_height.into(), Value::Auto))
                    // Keep clear of the Footer bar and the status line below it — the overlay's
                    // positioning has no idea the root layout put those there.
                    .with_style(OverlayInset::bottom(INPUT_GAP_HEIGHT + FOOTER_HEIGHT + STATUS_HEIGHT + 1))
                    // ...and clear of the Body's scrollbar on the right, for the same reason —
                    // the default 1-cell inset alone sits in the same column as the scrollbar.
                    .with_style(OverlayInset::right(2))
                    .with_style(Border::new(1, BorderStyle::Solid, Some(Color::new(overlay_pulse.0, overlay_pulse.1, overlay_pulse.2, 255))))
                    .with_style(BackgroundColor::new(BACKGROUND.0, BACKGROUND.1, BACKGROUND.2, 255))
                    // "0 1" — no vertical padding (the border alone gives enough breathing
                    // room top/bottom), 1 cell horizontal so text doesn't touch the border.
                    .with_style(Padding::left(1))
                    .with_style(Padding::right(1))
                    .with_content(Some(state.tasks_overlay_text(selected_task_index)))
            })
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
/// `with_overlay` because `Scaffold` only supports one detached overlay at a time (see
/// `with_overlay`'s doc comment in `escher_core::scaffold`), and the tasks overlay needed to keep
/// showing full time instead of swapping content.
struct AutocompleteBar;

/// A blank, contentless spacer slot between the `AutocompleteBar`/`Body` above and the `Footer`
/// input below — see `INPUT_GAP_HEIGHT`.
struct InputGap;
