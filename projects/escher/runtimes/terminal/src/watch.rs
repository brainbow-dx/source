//! External-event → wake bridge: background watchers for stdin readiness or OS signals that call
//! a plain `on_wake` callback the instant something arrives, instead of requiring whoever owns
//! the actual event loop to poll for it. Exists for a *reactive* event loop specifically (one
//! that only ticks in response to real events, e.g. `bevy_winit`'s own `WinitSettings::
//! desktop_app()`) — that kind of loop can't otherwise notice something that happened outside its
//! own event sources at all. `escher_bevy::terminal` is this module's only consumer today (its own
//! `spawn_input_watcher`/`spawn_signal_watcher`/`reraise_signal` are now thin wrappers around
//! these, supplying a winit-specific `on_wake`) — moved here, generalized off `winit::event_loop::
//! EventLoopProxy`, because none of the actual watching logic below is Bevy/winit-specific; only
//! "how to wake the loop once something happens" ever was.
//!
//! `TerminalApp::run`'s own signal handling (`app.rs`) is deliberately separate from this, not
//! reconciled into it — its own poll loop already checks a flag directly every iteration, so it
//! never had the "something external needs to force a tick" problem this module exists to solve.
//!
//! `on_wake` returns `bool`: `true` to keep watching, `false` to stop the background thread —
//! mirrors `EventLoopProxy::send_event`'s own `Result` (an error there means the event loop
//! itself is gone, so escher-bevy's wrapper closure returns `false` in exactly that case).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Polls stdin readiness on its own thread, calling `on_wake` immediately when input arrives (a
/// steady poll otherwise, once a second, doubling as a heartbeat wake even with nothing pending —
/// see this module's own doc comment for why a reactive event loop needs that). While input keeps
/// arriving, re-wakes roughly every 16ms rather than once — load-bearing, not a nicety, for a
/// consumer (`apps/anvil`'s own render loop) that caps how much it draws per tick and depends on
/// being ticked again promptly to keep draining a sustained burst instead of stalling partway.
pub fn spawn_input_watcher(on_wake: impl Fn() -> bool + Send + 'static) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::poll(std::time::Duration::from_secs(1)) {
                Ok(true) => {
                    if !on_wake() {
                        return;
                    }

                    let mut last_wake = std::time::Instant::now();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        match crossterm::event::poll(std::time::Duration::ZERO) {
                            Ok(true) => {
                                if last_wake.elapsed() >= std::time::Duration::from_millis(16) {
                                    if !on_wake() {
                                        return;
                                    }
                                    last_wake = std::time::Instant::now();
                                }
                            }
                            Ok(false) => break,
                            Err(_) => return,
                        }
                    }
                }
                Ok(false) => {
                    if !on_wake() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });
}

/// Watches for `SIGTERM`/`SIGHUP`/`SIGINT`, calling `on_wake` and logging (via `tracing::warn!`)
/// which signal arrived and, if the kernel reported one, the sending process's pid and name — so
/// a report of "something stopped Anvil" can say *what* stopped it, not just that it stopped.
/// Returns the flag a consumer should check each tick to know a signal fired and
/// which one; call [`reraise_signal`] with it after cleanup so the process actually terminates
/// with the signal's conventional exit status instead of returning as if this were a normal exit.
#[cfg(unix)]
pub fn spawn_signal_watcher(on_wake: impl Fn() + Send + 'static) -> Arc<AtomicUsize> {
    use signal_hook::iterator::exfiltrator::WithOrigin;
    use signal_hook::iterator::SignalsInfo;

    let flag = Arc::new(AtomicUsize::new(0));
    let mut signals = SignalsInfo::<WithOrigin>::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGHUP, signal_hook::consts::SIGINT])
        .expect("failed to register signal handler");

    let watcher_flag = flag.clone();
    std::thread::spawn(move || {
        for origin in signals.forever() {
            let signal_name = signal_hook::low_level::signal_name(origin.signal).unwrap_or("unknown signal");
            match origin.process {
                Some(process) => {
                    let sender_name = process_name(process.pid).unwrap_or_else(|| "unknown process".to_string());
                    tracing::warn!("Received {signal_name} from pid {} ({sender_name}) — shutting down", process.pid);
                }
                None => tracing::warn!("Received {signal_name} (sender unknown) — shutting down"),
            }

            watcher_flag.store(origin.signal as usize, Ordering::Relaxed);
            on_wake();
        }
    });

    flag
}

/// `ps`, not `/proc` — this crate's only Unix target today is macOS, which has no `/proc`. `-o
/// comm=` (no header, just the value) gives the plain executable name, matching what `pkill -f`/
/// `ps aux` themselves show. `None` covers both a real lookup failure and the mundane case where
/// the sending process has already exited by the time this runs (common for a one-shot `kill`).
#[cfg(unix)]
fn process_name(pid: i32) -> Option<String> {
    let output = std::process::Command::new("ps").args(["-p", &pid.to_string(), "-o", "comm="]).output().ok()?;
    let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Restores the default handler for whatever signal `flag` recorded (if any) and re-raises it —
/// call once, after terminal cleanup, at the very end of handling an exit a signal caused. A
/// no-op if `flag` is still `0` (a normal, non-signal exit).
#[cfg(unix)]
pub fn reraise_signal(flag: &AtomicUsize) {
    let sig = flag.load(Ordering::Relaxed);
    if sig != 0 {
        let _ = signal_hook::low_level::emulate_default_handler(sig as i32);
    }
}
