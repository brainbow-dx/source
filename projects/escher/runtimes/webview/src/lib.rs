//! Attaches a native browser webview to an existing OS window, given only that window's raw
//! handle. macOS only for now, built on `WKWebView` via `objc2`/`objc2-web-kit` — no equivalent
//! backend exists yet for Windows/Linux. Engine-agnostic: takes a `raw_window_handle::
//! RawWindowHandle`, not a Bevy (or any other engine's) window type, so any runtime that can
//! produce one can use this crate directly.

use raw_window_handle::RawWindowHandle;

#[cfg(target_os = "macos")]
mod macos;

/// Why [`WebView::attach`] failed.
#[derive(Debug)]
pub enum WebViewError {
    /// No backend exists for this platform/window handle type yet (only `RawWindowHandle::AppKit`
    /// on macOS is supported today).
    UnsupportedWindowHandle,
    /// AppKit calls are main-thread-only; `attach` wasn't called from it.
    NotOnMainThread,
    /// `url` failed to parse as an `NSURL`.
    InvalidUrl,
}

impl std::fmt::Display for WebViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebViewError::UnsupportedWindowHandle => write!(f, "no webview backend for this window handle type"),
            WebViewError::NotOnMainThread => write!(f, "must be called from the main thread"),
            WebViewError::InvalidUrl => write!(f, "invalid URL"),
        }
    }
}

impl std::error::Error for WebViewError {}

/// A plain, current desktop-Safari user-agent string — not applied automatically, callers opt in
/// via [`WebView::attach`]'s `user_agent` parameter. Exists because `WKWebView`'s own default
/// (unset `customUserAgent`, effectively "real Safari plus a WebKit-internal marker") is close to
/// real Safari's but not byte-identical, and this crate's caller is typically not a signed `.app`
/// bundle either — enough of a fingerprint gap that some sites' own browser-detection (Google
/// Search and YouTube, confirmed) route it to a reduced/legacy fallback UI, treating it as an
/// unrecognized/embedded browser. Spoofing this exact string is the standard workaround other apps
/// embedding `WKWebView` use. Real cost of opting in: the webview then reports as Safari to
/// *every* site it loads, not just the ones that needed it — there's no way to target the override
/// at specific hosts without intercepting every request, so it's a per-webview, not per-site,
/// choice.
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";

/// A live native webview attached to some host window. Dropping it removes the webview from the
/// window (platform-dependent — see the platform backend module for exact teardown behavior).
pub struct WebView {
    // Never read again after `attach` returns — its only job is to keep the underlying native
    // webview alive (and eventually drop it) for as long as this `WebView` is.
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    inner: macos::WebViewInner,
}

impl WebView {
    /// Attaches a new webview to `parent`'s window, loading `url`, leaving `top_inset` points of
    /// clear space at the top of the window (e.g. for a chrome bar attached separately) and
    /// `left_inset` points clear on the left (e.g. for a tab strip) — neither is this crate's own
    /// concern, `0.0`/`0.0` fills the whole window, the original behavior. The webview stays sized
    /// to fill whatever's left automatically as the window resizes (via the platform's own
    /// autoresizing mechanism) — no per-frame polling required from the caller.
    ///
    /// `user_agent`: `None` keeps `WKWebView`'s own default UA string unmodified. `Some(ua)` sets
    /// `customUserAgent` to exactly `ua` — pass [`DEFAULT_USER_AGENT`] for the common "make Google/
    /// YouTube/etc. serve their real UI instead of a reduced fallback" case (see its own doc
    /// comment for why that's needed and what it costs), or your own string if some other site
    /// needs different spoofing. Deliberately not defaulted inside this crate — which UA (if any)
    /// is right is an app-level policy choice, not something a reusable webview-attach crate should
    /// decide unasked.
    ///
    /// Must be called from the main thread — platform UI toolkits (AppKit included) require it.
    pub fn attach(parent: RawWindowHandle, url: &str, top_inset: f64, left_inset: f64, user_agent: Option<&str>) -> Result<Self, WebViewError> {
        #[cfg(target_os = "macos")]
        {
            macos::attach(parent, url, top_inset, left_inset, user_agent).map(|inner| WebView { inner })
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (parent, url, top_inset, left_inset, user_agent);
            Err(WebViewError::UnsupportedWindowHandle)
        }
    }

    /// Navigates the *existing* webview to `url` in place — no new native view, no re-attach.
    /// The one thing `attach` can't do: change what an already-open webview shows.
    pub fn load(&self, url: &str) -> Result<(), WebViewError> {
        #[cfg(target_os = "macos")]
        {
            self.inner.load(url)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = url;
            Err(WebViewError::UnsupportedWindowHandle)
        }
    }

    /// Steps back in this webview's own back/forward list (in-page navigation included, not just
    /// top-level loads — see `WKWebView.goBack`). A no-op if there's nowhere to go back to.
    pub fn go_back(&self) {
        #[cfg(target_os = "macos")]
        {
            self.inner.go_back();
        }
    }

    /// Steps forward — see `go_back`.
    pub fn go_forward(&self) {
        #[cfg(target_os = "macos")]
        {
            self.inner.go_forward();
        }
    }

    pub fn can_go_back(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            return self.inner.can_go_back();
        }
        #[cfg(not(target_os = "macos"))]
        false
    }

    pub fn can_go_forward(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            return self.inner.can_go_forward();
        }
        #[cfg(not(target_os = "macos"))]
        false
    }

    /// Whether the main frame is currently loading — a plain instant poll (not a push callback),
    /// backed by `WKNavigationDelegate` on macOS. Meant to be read every frame from wherever a
    /// caller already ticks (a toolbar's own redraw system, say), not subscribed to.
    pub fn is_loading(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            return self.inner.is_loading();
        }
        #[cfg(not(target_os = "macos"))]
        false
    }

    /// Shows or hides this webview in place — how multiple webviews sharing one window (one per
    /// browser tab, say) take turns being the visible one without tearing down and reattaching.
    pub fn set_hidden(&self, hidden: bool) {
        #[cfg(target_os = "macos")]
        {
            self.inner.set_hidden(hidden);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = hidden;
        }
    }

    /// Re-trims the left edge of an already-attached webview — for a host UI element (a tab strip)
    /// whose width can change after this webview was created, e.g. being collapsed/expanded.
    pub fn set_left_inset(&self, left_inset: f64) {
        #[cfg(target_os = "macos")]
        {
            self.inner.set_left_inset(left_inset);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = left_inset;
        }
    }
}
