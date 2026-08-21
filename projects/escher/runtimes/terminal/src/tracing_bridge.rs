//! Generic tracing-to-terminal-UI bridges — a bounded line buffer any terminal app can render as
//! a scrollback page, and the plumbing to feed one from real `tracing` output. Extracted from
//! `apps/anvil`'s `main.rs`: none of this is Anvil-specific — it's the same shape any
//! `escher-terminal`-hosted app would want for "show me a live tracing/log feed as a page," so it
//! belongs in this library, not the app.

use std::collections::VecDeque;
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use parking_lot::Mutex;

/// Forwards `tracing::*` events to a plain `mpsc::Sender<String>`, but only while a span named
/// `"live_trace"` is active somewhere in the current event's scope — the shape a UI wants for "the
/// user asked to watch this one command run," not the unscoped "everything, always" feed
/// `LineBuffer`'s own `MakeWriter` impl already covers via `tracing_subscriber::fmt::layer()`.
pub struct LiveTraceLayer {
    sender: std::sync::mpsc::Sender<String>,
}

impl LiveTraceLayer {
    pub fn new(sender: std::sync::mpsc::Sender<String>) -> Self {
        LiveTraceLayer { sender }
    }
}

impl<S> tracing_subscriber::Layer<S> for LiveTraceLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let in_live_trace = ctx.event_scope(event).is_some_and(|scope| scope.from_root().any(|span| span.name() == "live_trace"));

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

/// A bounded, thread-safe ring buffer of already-formatted lines (ANSI codes kept verbatim, not
/// stripped), oldest dropped once full at `capacity` (set at construction — callers running for a
/// whole process lifetime against a chatty feed want a real bound; a short-lived buffer might not).
#[derive(Clone)]
pub struct LineBuffer {
    lines: Arc<Mutex<VecDeque<String>>>,
    capacity: usize,
    /// Bumped once per `push_line` — a cheap, `O(1)` signal a caller can compare against a
    /// previous reading to tell "did this buffer actually change" without re-scanning its whole
    /// content. Added for `apps/anvil`'s Mario overlay: re-wrapping a full `snapshot()` to a
    /// fixed column width every frame, regardless of whether anything new was appended, is a
    /// real, measurable per-frame cost against a buffer at this capacity — this is what lets a
    /// caller cache that wrapped result and only redo it on a frame where the revision actually
    /// moved.
    revision: Arc<AtomicU64>,
}

impl LineBuffer {
    pub fn new(capacity: usize) -> Self {
        LineBuffer { lines: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))), capacity, revision: Arc::new(AtomicU64::new(0)) }
    }

    /// Appends one already-formatted line, dropping the oldest once at capacity.
    pub fn push_line(&self, line: String) {
        let mut lines = self.lines.lock();
        if lines.len() >= self.capacity {
            lines.pop_front();
        }
        lines.push_back(line);
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    /// See this struct's own doc comment on `revision`. `Relaxed` is enough — callers only ever
    /// compare this against a previously read value for equality, never order memory around it.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// A snapshot of everything currently retained, oldest first, joined into one string — ready
    /// to hand straight to `Scaffold::content`. Embedded ANSI is parsed by the terminal
    /// surface's own content rendering regardless of which node it came from, so no separate
    /// `ansi_to_tui` call is needed by callers of this.
    pub fn snapshot(&self) -> String {
        self.lines.lock().iter().cloned().collect::<Vec<_>>().join("\n")
    }

    /// The single most recent line, if any — for a "peek at the latest output" status indicator
    /// without switching a UI away from whatever page is currently showing.
    pub fn last_line(&self) -> Option<String> {
        self.lines.lock().back().cloned()
    }
}

/// A line-buffering `io::Write` sink for `tracing_subscriber::fmt::layer()`'s writer — the fmt
/// layer may split one event's output across more than one `write()` call, so this can't assume
/// one `write()` is one complete line. Bytes accumulate in `pending` until a `\n` completes a
/// line, at which point that whole line (ANSI included) is pushed into the shared `LineBuffer`.
pub struct LineBufferWriter {
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

/// A gate that writes straight to real stdout while `active` is `true`, discards otherwise —
/// backs a "leave the TUI for a plain scrolling raw-trace stream" mode any raw-mode terminal app
/// might want. Shares one `Arc<AtomicBool>` with whatever toggles the mode, so flipping that flag
/// takes effect on the very next write with no other plumbing.
#[derive(Clone)]
pub struct RawStreamGate {
    active: Arc<AtomicBool>,
}

impl RawStreamGate {
    pub fn new(active: Arc<AtomicBool>) -> Self {
        RawStreamGate { active }
    }
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
