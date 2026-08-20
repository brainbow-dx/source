//! Attaches a native browser webview to an existing OS window, given only that window's raw
//! handle. macOS only for now, built on `WKWebView` via `objc2`/`objc2-web-kit`. No equivalent
//! backend exists yet for Windows/Linux. Engine-agnostic: takes a `raw_window_handle::
//! RawWindowHandle`, not a Bevy (or any other engine's) window type, so any runtime that can
//! produce one can use this crate directly.

use std::sync::Arc;

use raw_window_handle::RawWindowHandle;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

/// One caller-supplied item appended to a link's native right-click context menu. See
/// [`WebView::attach`]'s `on_link_context_menu` parameter. `Arc<dyn Fn() + Send + Sync>`, not
/// `Box`, for the same reason `escher_os::menu::MenuItem::Item`'s action is: this needs to survive
/// being handed across into AppKit's target-action mechanism, which requires a stable, shareable
/// handle, not a one-shot `FnOnce`. Deliberately carries no built-in actions (no "Open in New Tab"
/// here). This crate only detects *that* a link was right-clicked and exposes its URL; deciding
/// what to actually offer is entirely the caller's job; see `apps/anvil`'s use site.
pub struct ContextMenuItem {
    pub label: String,
    pub action: Arc<dyn Fn() + Send + Sync>,
}

/// Registers a custom URL scheme (e.g. `"anvil"`, for `anvil://settings`) this webview serves
/// itself instead of asking the network for. See [`WebView::attach`]'s `custom_scheme`
/// parameter. `handler` is called with the full requested URL; `Some(html)` serves that HTML back
/// as the response, `None` fails the request (WebKit reports it as a load error, same as a real
/// 404 would read to the page). This crate has no opinion on what scheme or what content a caller
/// wants. Deciding that is entirely the caller's job, same reasoning as `ContextMenuItem`.
#[derive(Clone)]
pub struct CustomSchemeHandler {
    pub scheme: String,
    pub handler: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
}

/// Why [`WebView::attach`] failed.
#[derive(Debug)]
pub enum WebViewError {
    /// No backend exists for this platform/window handle type yet (only `RawWindowHandle::AppKit`
    /// on macOS and `RawWindowHandle::Win32` on Windows are supported today).
    UnsupportedWindowHandle,
    /// AppKit calls are main-thread-only; `attach` wasn't called from it.
    NotOnMainThread,
    /// `url` failed to parse (an `NSURL` on macOS; a plain navigate failure on Windows).
    InvalidUrl,
    /// A platform backend's own underlying call failed in a way none of the variants above
    /// already describe. Carries that failure's `Display` text verbatim rather than
    /// classifying every possible native error (COM `HRESULT`s on Windows, say) into its own
    /// variant. Windows-only today (`windows::attach`'s `EnvironmentBuilder::build` failing to
    /// even kick off the async environment/controller creation), but not gated to that platform.
    /// Any backend can reach for this instead of inventing a new variant per failure mode.
    PlatformError(String),
}

impl std::fmt::Display for WebViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebViewError::UnsupportedWindowHandle => write!(f, "no webview backend for this window handle type"),
            WebViewError::NotOnMainThread => write!(f, "must be called from the main thread"),
            WebViewError::InvalidUrl => write!(f, "invalid URL"),
            WebViewError::PlatformError(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for WebViewError {}

/// A plain, current desktop-Safari user-agent string. Not applied automatically, callers opt in
/// via [`WebView::attach`]'s `user_agent` parameter. Exists because `WKWebView`'s own default
/// (unset `customUserAgent`, effectively "real Safari plus a WebKit-internal marker") is close to
/// real Safari's but not byte-identical, and this crate's caller is typically not a signed `.app`
/// bundle either. That's enough of a fingerprint gap that some sites' own browser-detection (Google
/// Search and YouTube, confirmed) route it to a reduced/legacy fallback UI, treating it as an
/// unrecognized/embedded browser. Spoofing this exact string is the standard workaround other apps
/// embedding `WKWebView` use. Real cost of opting in: the webview then reports as Safari to
/// *every* site it loads, not just the ones that needed it. There's no way to target the override
/// at specific hosts without intercepting every request, so it's a per-webview, not per-site,
/// choice.
pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";

/// A live native webview attached to some host window. Dropping it removes the webview from the
/// window (platform-dependent; see the platform backend module for exact teardown behavior).
pub struct WebView {
    // Never read again after `attach` returns. Its only job is to keep the underlying native
    // webview alive (and eventually drop it) for as long as this `WebView` is.
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    inner: macos::WebViewInner,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    inner: windows::WebViewInner,
}

impl WebView {
    /// Attaches a new webview to `parent`'s window, loading `url`, leaving `top_inset` points of
    /// clear space at the top of the window (e.g. for a chrome bar attached separately) and
    /// `left_inset` points clear on the left (e.g. for a tab strip). Neither is this crate's own
    /// concern; `0.0`/`0.0` fills the whole window, the original behavior. The webview stays sized
    /// to fill whatever's left automatically as the window resizes (via the platform's own
    /// autoresizing mechanism), so no per-frame polling is required from the caller.
    ///
    /// `user_agent`: `None` keeps `WKWebView`'s own default UA string unmodified. `Some(ua)` sets
    /// `customUserAgent` to exactly `ua`. Pass [`DEFAULT_USER_AGENT`] for the common "make Google/
    /// YouTube/etc. serve their real UI instead of a reduced fallback" case (see its own doc
    /// comment for why that's needed and what it costs), or your own string if some other site
    /// needs different spoofing. Deliberately not defaulted inside this crate: which UA (if any)
    /// is right is an app-level policy choice, not something a reusable webview-attach crate should
    /// decide unasked.
    ///
    /// `on_link_context_menu`: called with a link's URL whenever the user right-clicks it,
    /// returning whatever extra items to append to WebKit's own default menu (Open Link, Copy
    /// Link, etc.). An empty `Vec` leaves that default menu untouched. Never called for a
    /// right-click that isn't over a link (image, plain text, ...); that case always gets WebKit's
    /// unmodified default menu, no hook available yet for it.
    ///
    /// `custom_scheme`: `None` registers nothing extra; every URL goes over the network as
    /// normal. `Some(handler)` makes this webview able to load `<handler.scheme>://...` URLs
    /// (typed in an address bar, navigated to in code, or loaded at attach time via `url` itself)
    /// by calling `handler.handler` instead of the network, same as any other in-app custom-scheme
    /// browser feature (an internal settings page, say).
    ///
    /// Must be called from the main thread. Platform UI toolkits (AppKit included) require it.
    pub fn attach(
        parent: RawWindowHandle,
        url: &str,
        top_inset: f64,
        left_inset: f64,
        user_agent: Option<&str>,
        on_link_context_menu: impl Fn(&str) -> Vec<ContextMenuItem> + 'static,
        custom_scheme: Option<CustomSchemeHandler>,
    ) -> Result<Self, WebViewError> {
        #[cfg(target_os = "macos")]
        {
            macos::attach(parent, url, top_inset, left_inset, user_agent, on_link_context_menu, custom_scheme).map(|inner| WebView { inner })
        }

        #[cfg(target_os = "windows")]
        {
            windows::attach(parent, url, top_inset, left_inset, user_agent, on_link_context_menu, custom_scheme).map(|inner| WebView { inner })
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (parent, url, top_inset, left_inset, user_agent, on_link_context_menu, custom_scheme);
            Err(WebViewError::UnsupportedWindowHandle)
        }
    }

    /// Navigates the *existing* webview to `url` in place. No new native view, no re-attach.
    /// The one thing `attach` can't do: change what an already-open webview shows.
    pub fn load(&self, url: &str) -> Result<(), WebViewError> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.inner.load(url)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = url;
            Err(WebViewError::UnsupportedWindowHandle)
        }
    }

    /// Steps back in this webview's own back/forward list (in-page navigation included, not just
    /// top-level loads; see `WKWebView.goBack`/`ICoreWebView2.GoBack`). A no-op if there's
    /// nowhere to go back to.
    pub fn go_back(&self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.inner.go_back();
        }
    }

    /// Steps forward. See `go_back`.
    pub fn go_forward(&self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.inner.go_forward();
        }
    }

    pub fn can_go_back(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            return self.inner.can_go_back();
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        false
    }

    pub fn can_go_forward(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            return self.inner.can_go_forward();
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        false
    }

    /// Whether the main frame is currently loading. A plain instant poll (not a push callback),
    /// backed by `WKNavigationDelegate` on macOS / `NavigationStarting`/`NavigationCompleted` on
    /// Windows. Meant to be read every frame from wherever a caller already ticks (a toolbar's
    /// own redraw system, say), not subscribed to.
    pub fn is_loading(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            return self.inner.is_loading();
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        false
    }

    /// The loaded page's own title, polled the same way as `is_loading` (see that method's own
    /// doc comment) rather than pushed via a callback. `None` on a platform with no backend, or
    /// before any page has set one yet.
    pub fn title(&self) -> Option<String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            return self.inner.title();
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        None
    }

    /// Shows or hides this webview in place. This is how multiple webviews sharing one window (one per
    /// browser tab, say) take turns being the visible one without tearing down and reattaching.
    pub fn set_hidden(&self, hidden: bool) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.inner.set_hidden(hidden);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = hidden;
        }
    }

    /// Re-trims the left edge of an already-attached webview. For a host UI element (a tab strip)
    /// whose width can change after this webview was created, e.g. being collapsed/expanded.
    pub fn set_left_inset(&self, left_inset: f64) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            self.inner.set_left_inset(left_inset);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = left_inset;
        }
    }
}
