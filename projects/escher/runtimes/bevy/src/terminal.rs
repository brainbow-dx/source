//! A real `escher-terminal` `Scaffold` UI drawn to the OS terminal the Bevy app was launched
//! from, not the game window itself, driven by Bevy's own `PreUpdate`/`Last` schedule instead
//! of a separate event loop. `TerminalSurface::draw` (see `escher_core::surface::Surface`)
//! polls `crossterm` itself each call, so nothing else in this plugin also polls it directly —
//! two independent pollers would race for the same event stream, each frame's event going to
//! whichever happened to run first.

use std::io::Stdout;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use parking_lot::RwLock;

use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::Event as BackendEvent;
use crossterm::event::KeyCode as TerminalKeyCode;
use crossterm::event::KeyEventKind;
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;

use ratatui::CompletedFrame;
use ratatui::Frame;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use escher_core::element::Footer;
use escher_core::element::Header;
use escher_core::style::Size;
use escher_core::style::TextAlign;
use escher_terminal::app::TerminalAction;
use escher_terminal::surface::TerminalSurface;

use bevy::app::AppExit;
use bevy::app::prelude::*;
use bevy::ecs::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::prelude::*;
use bevy::ecs::message::Message;

use crate::webview::SceneCommand;

pub struct TerminalPlugin;

impl TerminalPlugin {
    pub fn new() -> Self {
        TerminalPlugin
    }
}

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TerminalEvent>();
        app.add_message::<TerminalError>();

        app.insert_resource(TerminalProvider::new().expect("Terminal Provider"));

        app.add_systems(Startup, TerminalProvider::startup);
        app.add_systems(PreUpdate, TerminalProvider::draw_ui);
        app.add_systems(Last, TerminalProvider::exit_keys);
        app.add_systems(Last, TerminalProvider::exit);
    }
}

#[derive(Resource)]
pub struct TerminalProvider {
    terminal: TerminalSurface<CrosstermBackend<Stdout>>,
    /// What's been typed into the command line so far. Shared with the `CrosstermEvent` handler
    /// registered on the `Scaffold` each `draw_ui` call builds fresh — that handler runs inside
    /// `TerminalSurface::draw`'s own dispatch, not as a Bevy system, so it can't take a
    /// `MessageWriter` directly. It writes here instead; `draw_ui` reads it back to render.
    input: Arc<RwLock<String>>,
    /// Parsed `/scene <url>` commands, queued by that same handler for `draw_ui` to drain into
    /// real `MessageWriter<SceneCommand>` writes once it's back in normal system context.
    pending_scenes: Arc<Mutex<Vec<String>>>,
}

impl TerminalProvider {
    pub fn new() -> Result<Self, TerminalError> {
        Ok(TerminalProvider {
            terminal: TerminalSurface::try_default().map_err(|error| TerminalError::SurfaceError(error.to_string()))?,
            input: Arc::new(RwLock::new(String::new())),
            pending_scenes: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

impl TerminalProvider {
    pub fn setup(&mut self) -> Result<ExitCode, TerminalError> {
        crossterm::terminal::enable_raw_mode()?;

        match execute!(self.terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture) {
            Ok(_) => Ok(ExitCode::SUCCESS),
            Err(exec_error) => match self.drop() {
                Ok(_) => Err(TerminalError::from(exec_error)),
                Err(error) => Err(TerminalError::from(error)),
            },
        }
    }
}

impl TerminalProvider {
    pub fn drop(&mut self) -> Result<ExitCode, TerminalError> {
        if crossterm::terminal::is_raw_mode_enabled()? {
            let _ = crossterm::terminal::disable_raw_mode();
        }

        match execute!(self.terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture) {
            Ok(_) => {
                self.terminal.show_cursor()?;
                Ok(ExitCode::SUCCESS)
            }
            Err(error) => Err(TerminalError::from(error)),
        }
    }
}

impl TerminalProvider {
    pub fn draw<F>(&mut self, framer_fn: F) -> Result<CompletedFrame<'_>, TerminalError>
    where
        F: FnOnce(&mut Frame),
    {
        Ok(self.terminal.draw(framer_fn)?)
    }

    pub fn print<W: Widget + Clone>(&mut self, widget: W) -> bool {
        match self.draw(|frame| {
            frame.render_widget(widget, frame.area());
        }) {
            Ok(_) => true,
            Err(error) => {
                let message = Paragraph::new(format!("Failed to print to terminal: {error}"));

                match self.draw(move |frame| {
                    frame.render_widget(message, frame.area());
                }) {
                    Ok(_) => true,
                    Err(error) => panic!("Failed to print to terminal: {error}"),
                }
            }
        }
    }
}

impl TerminalProvider {
    fn startup(
        mut terminal: ResMut<TerminalProvider>,
        mut terminal_evt: MessageWriter<TerminalEvent>,
        event_loop_proxy: Res<bevy::winit::EventLoopProxyWrapper>,
    ) {
        match terminal.setup() {
            Ok(_) => {
                tracing::trace!("Terminal is ready!");
                terminal_evt.write(TerminalEvent::Ready);
                spawn_input_watcher(event_loop_proxy.clone());
            }
            Err(error) => {
                tracing::error!("Couldn't start terminal: {error}");
                tracing::debug!("Falling back to default stdout.");
                terminal_evt.write(TerminalEvent::StartupFailed(error));
            }
        }
    }

    /// Draws the terminal's `Scaffold` UI for this frame and dispatches whatever input arrived,
    /// via `TerminalSurface::draw_with_poll_timeout` — a zero timeout, not the library's own
    /// ~33ms default: this runs as a Bevy `PreUpdate` system, one call per tick, and Bevy's own
    /// reactive scheduling already decides when a tick is worth running at all, so blocking here
    /// too would stall Bevy's entire main thread on every tick that doesn't happen to have an
    /// event already waiting (see that method's own doc comment). A command line in the Footer is
    /// the whole UI for now: type `/scene <url>` and press Enter to fire a `SceneCommand` at the
    /// webview, proving terminal input and Bevy's own message system reach each other from inside
    /// the same schedule, not a second process.
    fn draw_ui(
        mut terminal: ResMut<TerminalProvider>,
        mut exit_evt: MessageWriter<AppExit>,
        mut scene_evt: MessageWriter<SceneCommand>,
    ) {
        let input = terminal.input.clone();
        let pending_scenes = terminal.pending_scenes.clone();

        let action = terminal.terminal.draw_with_poll_timeout({
            let handler_input = input.clone();
            let pending_scenes = pending_scenes.clone();

            move |root| {
                root.with_handler::<BackendEvent>(move |event| {
                    let input = &handler_input;
                    let BackendEvent::Key(key) = event else { return };

                    if key.kind == KeyEventKind::Release {
                        return;
                    }

                    match key.code {
                        TerminalKeyCode::Char(character) => input.write().push(character),
                        TerminalKeyCode::Backspace => {
                            input.write().pop();
                        }
                        TerminalKeyCode::Enter => {
                            let mut input = input.write();

                            if let Some(url) = input.strip_prefix("/scene ").map(str::trim).filter(|url| !url.is_empty()) {
                                pending_scenes.lock().unwrap().push(url.to_string());
                            }

                            input.clear();
                        }
                        _ => {}
                    }
                })
                .with_slot::<Header>(|header| {
                    header
                        .with_style(Size::height(1))
                        .with_style(TextAlign::Center)
                        .with_content(Some("Escher + Bevy, one process — /scene <url> to open a webview"))
                })
                .with_slot::<Footer>(|footer| footer.with_style(Size::height(1)).with_content(Some(format!("> {}", input.read()))))
            }
        }, Duration::ZERO);

        match action {
            Ok(TerminalAction::Exit(_)) => {
                exit_evt.write(AppExit::Success);
            }
            Ok(_) => {}
            Err(error) => tracing::warn!("Terminal draw failed: {error}"),
        }

        for url in pending_scenes.lock().unwrap().drain(..) {
            scene_evt.write(SceneCommand { url });
        }
    }

    fn exit_keys(keys: Res<ButtonInput<KeyCode>>, mut key_evt: MessageReader<KeyboardInput>, mut exit_evt: MessageWriter<AppExit>) {
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);

        for event in key_evt.read() {
            if ctrl && event.state == ButtonState::Pressed {
                exit_evt.write(AppExit::Success);
            }
        }
    }

    fn exit(mut terminal: ResMut<TerminalProvider>, mut exit_evt: MessageReader<AppExit>) {
        for _ in exit_evt.read() {
            terminal.print(Paragraph::new("Exiting. Goodbye! <3"));
            if let Err(error) = terminal.drop() {
                eprintln!("Failed to drop terminal: {error}");
            }
        }
    }
}

/// `WinitSettings::desktop_app()` (see `EscherBevyPlugin`) only runs Bevy's own schedule on a
/// throttled cadence, every 5s while the window has focus, every 60s while it doesn't, unless
/// something wakes it early. Winit has no visibility into this app's *other* input source, the
/// terminal's own stdin, so without this, a keystroke just sits unprocessed until the next
/// scheduled tick, up to a full minute of apparent unresponsiveness. This thread only *peeks*
/// (`crossterm::event::poll`, which checks readiness without consuming the event) so it never
/// races `draw_ui`'s own `poll`+`read` for the same event, it only tells winit there's real work
/// waiting.
///
/// After a successful peek, waits for the pending input to actually drain (re-peeking on a short
/// interval) before going back to the long idle poll, rather than a single fixed guess-sleep —
/// found this mattered for real: a flat 50ms post-wake sleep caps this thread's own re-check rate
/// at ~20Hz, which is also, in effect, an upper bound on how fast a fast typing burst can drain,
/// since each `WakeUp` here only guarantees one Bevy tick and each tick only dispatches whatever's
/// pending at that moment (see `assistant_terminal_draw`'s own drain-loop in `apps/anvil` for the
/// other half of this — this thread getting keystrokes noticed quickly doesn't help if the tick
/// it wakes doesn't also drain more than a couple of them). A still-short 2ms re-check interval
/// (not zero) keeps this from becoming a genuine tight busy-spin while still re-checking ~25x
/// faster than the old fixed sleep.
pub fn spawn_input_watcher(event_loop_proxy: winit::event_loop::EventLoopProxy<bevy::winit::WinitUserEvent>) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::poll(std::time::Duration::from_secs(1)) {
                Ok(true) => {
                    if event_loop_proxy.send_event(bevy::winit::WinitUserEvent::WakeUp).is_err() {
                        return;
                    }

                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        match crossterm::event::poll(std::time::Duration::ZERO) {
                            Ok(true) => continue,
                            Ok(false) => break,
                            Err(_) => return,
                        }
                    }
                }
                Ok(false) => {}
                Err(_) => return,
            }
        }
    });
}

/// Watches for `SIGTERM`/`SIGHUP`/`SIGINT` and wakes the event loop the moment one arrives,
/// returning the flag a consumer should check each tick (e.g. from a system also reading
/// `MessageWriter<AppExit>`) to know a signal fired and which one. Same shape and reasoning as
/// `spawn_input_watcher` above: `WinitSettings::desktop_app()` only ticks Bevy's `Update` schedule
/// reactively, so a bare `signal_hook::flag::register_usize` handler (which only sets an atomic
/// from inside the actual restricted signal-handler context, with no way to also wake anything)
/// could sit unnoticed for as long as that reactive mode's own idle-fallback interval — measured,
/// ~13s, not the near-instant response signal handling is supposed to give. Running on a real
/// background thread via `Signals::forever()` instead means it can call `EventLoopProxy::
/// send_event` directly, forcing an immediate tick; re-measured after this fix, ~125ms.
///
/// Call `reraise_signal` with the returned flag after cleanup, so the process actually terminates
/// with the signal's conventional exit status instead of returning as if this were a normal exit.
#[cfg(unix)]
pub fn spawn_signal_watcher(event_loop_proxy: winit::event_loop::EventLoopProxy<bevy::winit::WinitUserEvent>) -> Arc<std::sync::atomic::AtomicUsize> {
    let flag = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut signals = signal_hook::iterator::Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGHUP, signal_hook::consts::SIGINT])
        .expect("failed to register signal handler");

    let watcher_flag = flag.clone();
    std::thread::spawn(move || {
        for sig in signals.forever() {
            watcher_flag.store(sig as usize, std::sync::atomic::Ordering::Relaxed);
            let _ = event_loop_proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
        }
    });

    flag
}

/// Restores the default handler for whatever signal `flag` recorded (if any) and re-raises it —
/// call once, after terminal cleanup, at the very end of handling an `AppExit` a signal caused.
/// A no-op if `flag` is still `0` (a normal, non-signal exit).
#[cfg(unix)]
pub fn reraise_signal(flag: &std::sync::atomic::AtomicUsize) {
    let sig = flag.load(std::sync::atomic::Ordering::Relaxed);
    if sig != 0 {
        let _ = signal_hook::low_level::emulate_default_handler(sig as i32);
    }
}

#[derive(Message, Debug)]
pub enum TerminalEvent {
    StartupFailed(TerminalError),
    Ready,
    Backend(BackendEvent),
}

impl From<BackendEvent> for TerminalEvent {
    fn from(event: BackendEvent) -> Self {
        TerminalEvent::Backend(event)
    }
}

#[derive(Debug, Message)]
pub enum TerminalError {
    IoError(std::io::Error),
    /// `TerminalSurface::try_default()` failed — `color_eyre::Report` isn't `Clone`/`PartialEq`
    /// like a `Message` needs, so its display text is captured instead of the error itself.
    SurfaceError(String),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalError::IoError(error) => write!(f, "Std I/O Error: {error}"),
            TerminalError::SurfaceError(error) => write!(f, "Terminal surface error: {error}"),
        }
    }
}

impl std::error::Error for TerminalError {}

impl From<std::io::Error> for TerminalError {
    fn from(error: std::io::Error) -> Self {
        TerminalError::IoError(error)
    }
}
