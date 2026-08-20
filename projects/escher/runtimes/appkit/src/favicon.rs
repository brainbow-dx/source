//! Fetches and caches a favicon per host, the same `/favicon.ico`-by-convention approach every
//! browser falls back to when a page doesn't advertise a `<link rel="icon">` (which would need
//! parsing the page's own HTML — out of scope for a first pass; this covers the common case).
//!
//! `NSURLSession`'s completion handler can run on an arbitrary background thread, but `NSImage`
//! construction (and everything else AppKit) needs the main thread — so the handler does the
//! minimum thread-unsafe-adjacent thing: convert the response straight to a plain, `Send`-safe
//! `Vec<u8>` and drop it in a `Mutex`. The actual `NSImage` only ever gets built in `get()`, called
//! from `AppKitSurface::patch` on the main thread during a real `draw()` call.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::NSImage;
use objc2_foundation::{NSData, NSString, NSURL};

enum Entry {
    Loading,
    Loaded(Vec<u8>),
    Failed,
}

pub struct FaviconCache {
    /// Raw fetched bytes, shared with (and written by) the background completion handler.
    bytes: Arc<Mutex<HashMap<String, Entry>>>,
    /// Decoded `NSImage`s, main-thread-only (never touched off it), so this stays a plain
    /// `HashMap` rather than needing the same `Mutex` the byte cache does.
    images: HashMap<String, Retained<NSImage>>,
}

impl FaviconCache {
    pub fn new() -> Self {
        FaviconCache { bytes: Arc::new(Mutex::new(HashMap::new())), images: HashMap::new() }
    }

    /// Returns the cached favicon for `host`, kicking off a fetch the first time it's asked for
    /// and returning `None` (a caller should just show no image / a placeholder) until that
    /// completes — never blocks.
    pub fn get(&mut self, host: &str) -> Option<Retained<NSImage>> {
        if let Some(image) = self.images.get(host) {
            return Some(Retained::clone(image));
        }

        let mut bytes = self.bytes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        match bytes.get(host) {
            Some(Entry::Loaded(data)) => {
                let data = data.clone();
                drop(bytes);
                let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&data));
                if let Some(image) = image {
                    self.images.insert(host.to_string(), Retained::clone(&image));
                    Some(image)
                } else {
                    None
                }
            }
            Some(Entry::Loading) | Some(Entry::Failed) => None,
            None => {
                bytes.insert(host.to_string(), Entry::Loading);
                drop(bytes);
                self.fetch(host.to_string());
                None
            }
        }
    }

    fn fetch(&self, host: String) {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(&format!("https://{host}/favicon.ico"))) else {
            return;
        };

        let bytes = self.bytes.clone();
        let handler_host = host.clone();

        let completion = block2::RcBlock::new(move |data: *mut NSData, _response: *mut objc2_foundation::NSURLResponse, error: *mut objc2_foundation::NSError| {
            let mut bytes = bytes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

            if error.is_null() && !data.is_null() {
                // SAFETY: non-null per the completion handler's own contract when `error` is null.
                let data = unsafe { &*data };
                bytes.insert(handler_host.clone(), Entry::Loaded(data.to_vec()));
            } else {
                bytes.insert(handler_host.clone(), Entry::Failed);
            }
        });

        // SAFETY: `completion` is a real `block2::RcBlock`, sendable (owns only `Send` data —
        // an `Arc<Mutex<..>>` clone and a `String`), matching the binding's own safety contract.
        unsafe {
            let session = objc2_foundation::NSURLSession::sharedSession();
            let task = session.dataTaskWithURL_completionHandler(&url, &completion);
            task.resume();
        }
    }
}
