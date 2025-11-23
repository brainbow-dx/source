//! Disclaimer: This is (lightly modified) AI slop. I didn't want to write
//! the subscriber by hand while solving more interesting domain problems,
//! so I took a shortcut. This should *not* make it to the public repo.
//!
//! Note: if you see this in the public repo, pls open a PR and berate
//! the repo maintainer (or whoever).
#![allow(unused)]

#[cfg(not(debug_assertions))]
compile_error!("DO NOT USE THIS; AI-GENERATED SLOP");

//---    
use core::fmt::Write;
use core::fmt::Debug;
use core::cell::RefCell;
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicUsize;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use hashbrown::HashMap;

use parking_lot::Mutex;
use parking_lot::RwLock;

use color_eyre::owo_colors::OwoColorize;

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

#[derive(Default, Clone)]
pub struct TracingSubscriber<B> {
    entries: Arc<RwLock<VecDeque<B>>>,
    spans: Arc<RwLock<HashMap<Id, SpanData>>>,
}

thread_local! {
    // TODO: This is actual garbage. Rewrite it.
    static CURRENT_SPAN: RefCell<Vec<Id>> = RefCell::new(Vec::new());
}

impl<B> From<Arc<RwLock<VecDeque<B>>>> for TracingSubscriber<B> {
    fn from(entries: Arc<RwLock<VecDeque<B>>>) -> Self {
        TracingSubscriber {
            entries,
            spans: Arc::default()
        }
    }
}

impl<B> TracingSubscriber<B> {
    pub fn with_capacity(capacity: usize) -> Self {
        TracingSubscriber {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl TracingSubscriber<String> {
    // TODO: Move this to the TracingStream formatter itself.
    pub fn sample_end(&self, rows: usize, columns: usize) -> String {
        let capacity = rows * (columns + rows);
        let mut frame = String::with_capacity(capacity);
        
        let tracing_stream = self.entries().read();
        let lines = tracing_stream.iter().rev().take(rows).rev();
        
        for line in lines {
            frame.push_str(line);
            frame.push('\n');
        }
        
        if frame.ends_with('\n') {
            frame.pop();
        }
                
        frame
    }
}

impl<B> TracingSubscriber<B> {
    pub fn entries(&self) -> &Arc<RwLock<VecDeque<B>>> {
        &self.entries
    }
}

impl Subscriber for TracingSubscriber<String> {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= Level::TRACE
    }
    
    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let id = Id::from_u64(0);
        let span = SpanData::new(attrs, attrs.parent().cloned());
        self.spans.write().insert(id.clone(), span);
        id
    }
    
    fn record(&self, id: &Id, record: &Record<'_>) {
        if let Some(data) = self.spans.read().get(id) {
            let mut values = data.values.lock();
            let mut visitor = SpanFieldVisitor(&mut values);
            record.record(&mut visitor);
        }
    }
    
    fn record_follows_from(&self, span: &Id, follows: &Id) {
        //..
    }
    
    fn enter(&self, span_id: &Id) {
        CURRENT_SPAN.with(|spans| spans.borrow_mut().push(span_id.clone()));
    }
    
    fn exit(&self, span_id: &Id) {
        CURRENT_SPAN.with(|spans| {
            if let Some(span) = spans.borrow_mut().pop() {
                debug_assert_eq!(&span, span_id);
            }
        });
    }
    
    fn clone_span(&self, span_id: &Id) -> Id {
        if let Some(span_data) = self.spans.read().get(span_id) {
            span_data.ref_count.fetch_add(1, Ordering::Relaxed);
        }
        span_id.to_owned()
    }
    
    fn try_close(&self, span_id: Id) -> bool {
        if let Some(span) = self.spans.read().get(&span_id) {
            if span.ref_count.fetch_sub(1, Ordering::Release) == 1 {
                let mut spans = self.spans.write();
                spans.remove(&span_id);
                return true;
            }
        }
        false
    }
    
    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let level_styled = match *metadata.level() {
            Level::ERROR => "ERR".red().into_styled(),
            Level::WARN => "WRN".yellow().into_styled(),
            Level::INFO => "INF".blue().into_styled(),
            Level::DEBUG => "DBG".magenta().into_styled(),
            Level::TRACE => "TRC".cyan().into_styled(),
        };
        
        let mut message = String::with_capacity(120);
        let mut entry = EventFieldVisitor(&mut message);
        
        event.record(&mut entry);
        
        let mut entries = self.entries.write();
        
        if entries.len() >= entries.capacity() {
            entries.pop_front();
        }
        
        entries.push_back(format!("{} {} {}", level_styled, metadata.target().dimmed(), message));
    }
}

//---
// --- Omitted for brevity, but unchanged from previous response ---
struct SpanData {
    values: Mutex<HashMap<&'static str, String>>,
    metadata: &'static Metadata<'static>,
    ref_count: AtomicUsize,
    parent: Option<Id>,
}

impl SpanData {
    fn new(attrs: &Attributes<'_>, parent: Option<Id>) -> Self {
        let mut values = HashMap::new();
        let mut visitor = SpanFieldVisitor(&mut values);
        attrs.record(&mut visitor);
        Self {
            values: Mutex::new(values),
            metadata: attrs.metadata(),
            ref_count: AtomicUsize::new(1),
            parent,
        }
    }
}

pub struct SpanFieldVisitor<'visitor>(&'visitor mut HashMap<&'static str, String>);

impl<'a> Visit for SpanFieldVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0.insert(field.name(), format!("{:?}", value));
    }
    
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name(), value.to_string());
    }
}

//---
pub struct EventFieldVisitor<'visitor>(&'visitor mut String);

impl<'visitor> Visit for EventFieldVisitor<'visitor> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            write!(self.0, "{:?}", value).unwrap();
        } else {
            write!(self.0, " {}: {:?}", field.name(), value).unwrap();
        }
    }
}
