use core::time::Duration;

// TODO: Restrict this to the std feature and find a
// fallback for wasm, embedded, ffi, etc.
use std::io::Stdout;

use eyre::Error;

use ratatui::prelude::Backend;
use ratatui::prelude::CrosstermBackend;

use crate::surface::TerminalSurface;

#[derive(Debug)]
pub struct TerminalApp<B: Backend = CrosstermBackend<Stdout>> {
    surface: Option<TerminalSurface<B>>,
    tick_speed: Duration,
}

impl<B: Backend> TerminalApp<B> {
    pub fn new() -> Self {
        TerminalApp::<B> {
            surface: None,
            tick_speed: Duration::from_millis(16),
        }
    }
    
    pub fn with_surface(mut self, surface: TerminalSurface<B>) -> Self {
        self.surface = Some(surface);
        self // ..
    }
    
    pub fn with_tick_speed(mut self, speed: Duration) -> Self {
        self.tick_speed = speed;
        self // ..
    }
}

impl TerminalApp<CrosstermBackend<Stdout>> {
    pub fn run<F>(self, draw_surface_fn: F) -> Result<(), Error>
    where
        F: Fn(&mut TerminalSurface<CrosstermBackend<Stdout>>) -> Result<TerminalAction, Error>,
    {
        if let Some(mut surface) = self.surface {
            let mut stdout = surface.stdout();

            // Without raw mode, the terminal stays in canonical (line-buffered, echoing) mode:
            // keystrokes get echoed by the tty itself instead of only being drawn by the app,
            // and crossterm only sees input once a line is flushed rather than per-keystroke.
            crossterm::terminal::enable_raw_mode()?;

            // Draws to a dedicated alternate screen buffer instead of the terminal's live
            // scrollback — without this, the app draws directly over whatever the user had on
            // screen (their shell history, prior command output), and there's nothing for
            // `restore_terminal` to swap back to on exit. This has to happen before installing
            // the panic hook below, so a panic during setup still restores correctly.
            crossterm::execute!(&mut stdout, crossterm::terminal::EnterAlternateScreen)?;

            // A panic inside `draw_surface_fn` (or anywhere in the loop below) would otherwise
            // unwind straight past the teardown code at the end of this function, leaving the
            // terminal stuck in raw mode on the alternate screen — the shell prompt would still
            // come back eventually, but garbled, uneditable, and hidden behind whatever was last
            // drawn. Restoring first and re-raising through the previous hook keeps both: a
            // usable terminal *and* the original panic message/backtrace.
            let previous_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore_terminal();
                previous_hook(info);
            }));

            // The panic hook above only covers in-process Rust panics. An external `kill`/
            // `pkill` (no `-9`) sends `SIGTERM` by default, which nothing here handled before —
            // the process died with raw mode and mouse capture still enabled, leaving whatever
            // shell regains control reading unbuffered escape sequences instead of lines.
            // `SIGKILL` genuinely can't be caught by any process, so it's out of scope here;
            // `SIGTERM`/`SIGHUP`/`SIGINT` are the realistic, catchable cases. `register_usize`
            // (not the plain bool `register`) so the loop below knows *which* signal fired, to
            // re-raise the same one on the way out rather than always exiting as if it were
            // `SIGTERM`. The handler itself only does an atomic store — async-signal-safe by
            // construction, no `unsafe` needed on our side.
            #[cfg(unix)]
            let signal_flag = {
                use std::sync::Arc;
                use std::sync::atomic::AtomicUsize;

                let flag = Arc::new(AtomicUsize::new(0));
                for sig in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGHUP, signal_hook::consts::SIGINT] {
                    signal_hook::flag::register_usize(sig, Arc::clone(&flag), sig as usize)?;
                }
                flag
            };

            crossterm::execute!(
                &mut stdout,
                crossterm::event::EnableFocusChange,
                crossterm::event::EnableMouseCapture,
                crossterm::event::EnableBracketedPaste,
                crossterm::cursor::EnableBlinking,
            )?;

            surface.clear()?;

            loop {
                #[cfg(unix)]
                {
                    let sig = signal_flag.load(std::sync::atomic::Ordering::Relaxed);
                    if sig != 0 {
                        #[cfg(feature="dev")]
                        tracing::debug!("Received signal {sig}, restoring terminal and exiting ..");
                        break;
                    }
                }

                match draw_surface_fn(&mut surface) {
                    Ok(action) => match action {
                        #[allow(unused)]
                        TerminalAction::Exit(code) => {
                            #[cfg(feature="dev")]
                            tracing::debug!("Exiting with code '{:}' ..", code);
                            break; // <3
                        }
                        TerminalAction::EmptyCopyAttempt | TerminalAction::Copied => {
                            // No app-agnostic behavior here — a caller with real state (see
                            // `AppState::mouse_trouble_since` in the assistant example) reacts to
                            // this before it ever reaches this generic loop.
                            continue;
                        }
                        TerminalAction::NoOp => {
                            #[cfg(all(feature="dev", feature="verbose"))]
                            tracing::trace!("No-op ..");
                            continue;
                        }
                    }
                    Err(error) => {
                        tracing::error!("Failed to draw TerminalApp surface: {:}", error);
                        break; // </3
                    }
                }
            }
            
            crossterm::execute!(
                &mut stdout,
                crossterm::event::DisableFocusChange,
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableBracketedPaste,
                crossterm::cursor::DisableBlinking,
            )?;

            // No `surface.clear()`/`set_cursor_position` call needed here — leaving the
            // alternate screen below restores whatever the user had on screen before this app
            // started, cursor position included. Clearing first would only be clearing the
            // alternate screen that's about to be discarded anyway.
            //
            // The panic hook installed above is left in place rather than reverted — it just
            // calls `restore_terminal()` (a safe no-op on an already-restored terminal) before
            // delegating to the real original hook, same as ratatui's own `set_panic_hook`.
            restore_terminal();

            // Whatever sent the signal (or the shell that ran `kill`) expects this process to
            // actually go away, not linger after cleanup — restores the signal's default
            // disposition and re-raises it, so the process exits with the conventional 128+sig
            // status instead of silently returning `Ok(())` as if this were a normal in-app
            // `TerminalAction::Exit`.
            #[cfg(unix)]
            {
                let sig = signal_flag.load(std::sync::atomic::Ordering::Relaxed);
                if sig != 0 {
                    let _ = signal_hook::low_level::emulate_default_handler(sig as i32);
                }
            }
        };

        Ok(())
    }
}

/// Disables raw mode and leaves the alternate screen, restoring whatever the terminal displayed
/// before `TerminalApp::run` started it. Safe to call more than once (both the normal exit path
/// and the panic hook call it) — disabling raw mode or leaving the alternate screen when already
/// in that state is a no-op for crossterm, not an error.
fn restore_terminal() {
    use std::io::stdout;
    use std::io::Write;

    if let Err(error) = crossterm::terminal::disable_raw_mode() {
        eprintln!("Failed to disable raw mode: {error}");
    }

    // Reported bug: after exiting, the shell prompt comes back shifted down one line,
    // persistently, until the terminal is closed and reopened — `EnterAlternateScreen`/
    // `LeaveAlternateScreen` (mode 1049) is supposed to save/restore cursor position
    // automatically, so this points at some other piece of terminal state (most likely a
    // scroll region — DECSTBM — or a leftover SGR attribute) being left non-default rather
    // than the alternate-screen switch itself. Explicitly resetting the scroll region to the
    // full screen (`\x1b[r`) and all SGR attributes (`\x1b[0m`) *before* leaving the alternate
    // screen is a standard defensive hardening for exactly this class of bug — belt-and-braces
    // on top of what `LeaveAlternateScreen` already does, not a replacement for it.
    let _ = crossterm::execute!(stdout(), crossterm::style::ResetColor);
    let _ = write!(stdout(), "\x1b[r\x1b[0m");
    let _ = stdout().flush();

    if let Err(error) = crossterm::execute!(stdout(), crossterm::terminal::LeaveAlternateScreen) {
        eprintln!("Failed to leave alternate screen: {error}");
    }
}

#[derive(Default, Debug)]
pub enum TerminalAction {
    Exit(i8),
    /// A copy shortcut fired but nothing was selected. A real signal a caller can use to warn
    /// that mouse input may not be reaching the app, e.g. the terminal emulator intercepted the
    /// click/drag itself instead of forwarding it.
    EmptyCopyAttempt,
    /// A copy shortcut fired and something was actually copied.
    Copied,
    #[default]
    NoOp,
}
