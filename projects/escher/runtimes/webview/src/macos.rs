//! macOS backend: attaches a `WKWebView` to an `NSView` via `objc2`/`objc2-web-kit`. Sized with
//! AppKit's own autoresizing mask (`ViewWidthSizable | ViewHeightSizable`) so the OS keeps it
//! filling its parent view (below `top_inset`, if any — see `attach`) with no per-frame polling
//! needed from the caller.
//!
//! Dark by construction, not by preference: the webview's `NSAppearance` is forced to Dark Aqua
//! (not "match the OS", which could still be light) — the real, WebKit-native mechanism that makes
//! `prefers-color-scheme: dark` report true to any site that implements it, the same way Safari's
//! own dark mode works, no CSS injection involved. A site with no dark mode support at all just
//! renders its normal light theme — this used to also force a blanket CSS invert-filter onto every
//! page regardless (a common "poor man's dark mode" trick), but that broke plenty of real sites
//! outright (inverting a page that was never designed to be inverted is not the same as a site's
//! own, real dark theme) and double-inverted the sites that *did* support `prefers-color-scheme`
//! properly — removed. `underPageBackgroundColor` is still set dark, so the brief blank moment
//! before a page paints anything is dark instead of a bright white flash.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly, Message};

use objc2_app_kit::NSAppearance;
use objc2_app_kit::NSAppearanceCustomization;
use objc2_app_kit::NSAppearanceNameDarkAqua;
use objc2_app_kit::NSAutoresizingMaskOptions;
use objc2_app_kit::NSColor;
use objc2_app_kit::NSView;

use objc2_foundation::NSError;
use objc2_foundation::NSObject;
use objc2_foundation::NSObjectProtocol;
use objc2_foundation::NSString;
use objc2_foundation::NSURL;
use objc2_foundation::NSURLRequest;

use objc2_web_kit::WKNavigation;
use objc2_web_kit::WKNavigationDelegate;
use objc2_web_kit::WKWebView;
use objc2_web_kit::WKWebViewConfiguration;

use raw_window_handle::RawWindowHandle;

use crate::WebViewError;

/// Flips `flag` on `didStartProvisionalNavigation:`/off on any of the three ways a main-frame
/// navigation can end (finish, or fail either before or after it committed) — a plain instant
/// signal `WebViewInner::is_loading` polls, not a push callback, so `escher-bevy`/app code that
/// already ticks every frame (`sync_toolbar_state` in `apps/anvil`, say) can just read it alongside
/// everything else it's already re-checking, rather than needing new event plumbing threaded
/// through `escher-bevy`'s webview plugin. Exists because nothing anywhere in this app previously
/// showed *any* feedback while a page was loading — the toolbar just sat there looking identical
/// whether a click had registered or not, which read as "clunky."
pub struct NavigationDelegateIvars {
    loading: Arc<AtomicBool>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `NavigationDelegate` doesn't implement
    // `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = NavigationDelegateIvars]
    struct NavigationDelegate;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for NavigationDelegate {}

    // SAFETY: every method below matches `WKNavigationDelegate`'s real selector signature exactly;
    // none of them touch `navigation`/`error` beyond ignoring them, so their exact validity is
    // never relied on.
    unsafe impl WKNavigationDelegate for NavigationDelegate {
        #[unsafe(method(webView:didStartProvisionalNavigation:))]
        fn started(&self, _web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            self.ivars().loading.store(true, Ordering::Relaxed);
        }

        #[unsafe(method(webView:didFinishNavigation:))]
        fn finished(&self, _web_view: &WKWebView, _navigation: Option<&WKNavigation>) {
            self.ivars().loading.store(false, Ordering::Relaxed);
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn failed_provisional(&self, _web_view: &WKWebView, _navigation: Option<&WKNavigation>, _error: &NSError) {
            self.ivars().loading.store(false, Ordering::Relaxed);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn failed(&self, _web_view: &WKWebView, _navigation: Option<&WKNavigation>, _error: &NSError) {
            self.ivars().loading.store(false, Ordering::Relaxed);
        }
    }
);

impl NavigationDelegate {
    fn new(mtm: MainThreadMarker, loading: Arc<AtomicBool>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(NavigationDelegateIvars { loading });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}

/// Forces the `WKWebView`'s own `NSAppearance` to Dark Aqua — see this module's doc comment for
/// why "force", not "match the OS" (the previous behavior). Set once at attach time; live-
/// updating if the user toggles system appearance while the webview is already open isn't handled
/// here (it wouldn't matter now anyway, since this is no longer OS-preference-driven).
fn force_dark_appearance(webview: &WKWebView) {
    // SAFETY: reading a `&'static NSAppearanceName` extern static; no preconditions beyond being
    // linked against AppKit, which this whole module already requires.
    let dark_aqua = unsafe { NSAppearanceNameDarkAqua };

    if let Some(appearance) = NSAppearance::appearanceNamed(dark_aqua) {
        webview.setAppearance(Some(&appearance));
    }
}

pub struct WebViewInner {
    handle: Retained<WKWebView>,
    /// Kept around (not just borrowed at `attach` time) so `set_left_inset` can re-read its
    /// *current* frame — needed to recompute `handle`'s frame against the parent's live size
    /// rather than whatever size it happened to be at attach time.
    host: Retained<NSView>,
    top_inset: f64,
    loading: Arc<AtomicBool>,
    /// `setNavigationDelegate` is a *weak* property (see `objc2_web_kit`'s own doc comment on it)
    /// — nothing else keeps this alive, so without holding it here it would be deallocated the
    /// instant `attach` returns and no navigation callback would ever fire again.
    _navigation_delegate: Retained<NavigationDelegate>,
}

impl WebViewInner {
    pub fn load(&self, url: &str) -> Result<(), WebViewError> {
        let Some(parsed_url) = NSURL::URLWithString(&NSString::from_str(url)) else {
            return Err(WebViewError::InvalidUrl);
        };

        let request = NSURLRequest::requestWithURL(&parsed_url);
        unsafe { self.handle.loadRequest(&request) };
        tracing::info!("WebView loading {url}");

        Ok(())
    }

    pub fn go_back(&self) {
        unsafe { self.handle.goBack() };
    }

    pub fn go_forward(&self) {
        unsafe { self.handle.goForward() };
    }

    pub fn can_go_back(&self) -> bool {
        unsafe { self.handle.canGoBack() }
    }

    pub fn can_go_forward(&self) -> bool {
        unsafe { self.handle.canGoForward() }
    }

    pub fn set_hidden(&self, hidden: bool) {
        self.handle.setHidden(hidden);
    }

    /// Whether the main frame is currently between `didStartProvisionalNavigation:` and
    /// finishing/failing — see `NavigationDelegate`'s own doc comment.
    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Relaxed)
    }

    /// Re-trims the left edge (e.g. a tab strip being collapsed/expanded after this webview was
    /// already attached) — same inset math `attach` uses, just re-run against the host's current
    /// frame instead of a one-time snapshot of it.
    pub fn set_left_inset(&self, left_inset: f64) {
        let host_frame = self.host.frame();
        let is_flipped = self.host.isFlipped();
        let frame = objc2_foundation::NSRect {
            origin: objc2_foundation::NSPoint { x: left_inset, y: if is_flipped { self.top_inset } else { 0.0 } },
            size: objc2_foundation::NSSize { width: host_frame.size.width - left_inset, height: host_frame.size.height - self.top_inset },
        };
        self.handle.setFrame(frame);
    }
}

pub fn attach(parent: RawWindowHandle, url: &str, top_inset: f64, left_inset: f64, user_agent: Option<&str>) -> Result<WebViewInner, WebViewError> {
    let RawWindowHandle::AppKit(appkit_handle) = parent else {
        return Err(WebViewError::UnsupportedWindowHandle);
    };

    let mtm = MainThreadMarker::new().ok_or(WebViewError::NotOnMainThread)?;

    // SAFETY: the caller is responsible for `parent` staying valid for as long as the returned
    // `WebView` is alive — the same contract `raw-window-handle` itself places on any
    // `RawWindowHandle` consumer.
    let ns_view: &NSView = unsafe { appkit_handle.ns_view.cast().as_ref() };

    // Starts `top_inset` points short of the parent's full height. Which end gets the `y` offset
    // depends on whether `parent` uses AppKit's default coordinate system (origin bottom-left, y
    // increasing upward — leaving room at the top means trimming height, not offsetting `y`) or a
    // *flipped* one (origin top-left, y increasing downward — e.g. `bevy_winit`'s own content
    // view; leaving room at the top there means offsetting `y` *by* `top_inset`, not trimming from
    // the bottom). Getting this wrong doesn't error or misposition subtly — it silently reserves
    // the gap at the *wrong end* of the window, so the inset appears to do nothing at the top
    // (this exact bug, already found and fixed for the chrome bar's own `escher_os::macos::chrome
    // ::attach` — never applied here, which is why the top inset stopped visibly working the
    // moment this got attached to a real `bevy_winit` window instead of the non-flipped `NSWindow`
    // this was presumably last visually checked against).
    //
    // Paired with the `ViewHeightSizable` mask below (not `ViewMinYMargin`/`ViewMaxYMargin`, which
    // would instead keep this fixed-size and let the *gap* grow): as the window resizes, this
    // view's own height changes to absorb the difference while both margins — the `top_inset`-tall
    // gap and its flush-zero gap on the other side — stay constant, regardless of which one is
    // geometrically "at the top" for this particular parent.
    // `left_inset` (e.g. reserving space for a tab strip) is unaffected by flippedness — x
    // increases rightward regardless — so it's just a plain trim off the left edge, paired with
    // `ViewWidthSizable` the same way `top_inset` is paired with `ViewHeightSizable` below.
    let parent_frame = ns_view.frame();
    let is_flipped = ns_view.isFlipped();
    let inset_frame = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint { x: left_inset, y: if is_flipped { top_inset } else { 0.0 } },
        size: objc2_foundation::NSSize { width: parent_frame.size.width - left_inset, height: parent_frame.size.height - top_inset },
    };

    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };

    let webview = unsafe { WKWebView::initWithFrame_configuration(mtm.alloc(), inset_frame, &configuration) };

    webview.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);

    // `None` leaves `WKWebView`'s own default UA alone; see `user_agent`'s own doc comment on
    // `WebView::attach` (and `DEFAULT_USER_AGENT`'s, if the caller opted into that one) for why a
    // caller might pass something here at all.
    if let Some(user_agent) = user_agent {
        let user_agent = NSString::from_str(user_agent);
        unsafe { webview.setCustomUserAgent(Some(&user_agent)) };
    }

    force_dark_appearance(&webview);
    unsafe { webview.setUnderPageBackgroundColor(Some(&NSColor::blackColor())) };

    let loading = Arc::new(AtomicBool::new(false));
    let navigation_delegate = NavigationDelegate::new(mtm, loading.clone());
    unsafe { webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*navigation_delegate))) };

    let inner = WebViewInner { handle: webview, host: ns_view.retain(), top_inset, loading, _navigation_delegate: navigation_delegate };
    inner.load(url)?;

    ns_view.addSubview(&inner.handle);

    Ok(inner)
}
