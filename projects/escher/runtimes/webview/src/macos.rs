//! macOS backend: attaches a `WKWebView` to an `NSView` via `objc2`/`objc2-web-kit`. Sized with
//! AppKit's own autoresizing mask (`ViewWidthSizable | ViewHeightSizable`) so the OS keeps it
//! filling its parent view (below `top_inset`, if any; see `attach`) with no per-frame polling
//! needed from the caller.
//!
//! Dark by construction, not by preference: the webview's `NSAppearance` is forced to Dark Aqua
//! (not "match the OS", which could still be light). This is the real, WebKit-native mechanism
//! that makes `prefers-color-scheme: dark` report true to any site that implements it, the same
//! way Safari's own dark mode works, no CSS injection involved. A site with no dark mode support
//! at all just renders its normal light theme. This used to also force a blanket CSS invert-filter
//! onto every page regardless (a common "poor man's dark mode" trick), but that broke plenty of
//! real sites outright (inverting a page that was never designed to be inverted is not the same as
//! a site's own, real dark theme) and double-inverted the sites that *did* support
//! `prefers-color-scheme` properly, so it was removed. `underPageBackgroundColor` is still set
//! dark, so the brief blank moment before a page paints anything is dark instead of a bright white
//! flash.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message};

use objc2_app_kit::NSAppearance;
use objc2_app_kit::NSAppearanceCustomization;
use objc2_app_kit::NSAppearanceNameDarkAqua;
use objc2_app_kit::NSAutoresizingMaskOptions;
use objc2_app_kit::NSColor;
use objc2_app_kit::NSMenu;
use objc2_app_kit::NSMenuItem;
use objc2_app_kit::NSView;
use objc2_app_kit::NSWindowOrderingMode;

use objc2_foundation::NSData;
use objc2_foundation::NSError;
use objc2_foundation::NSObject;
use objc2_foundation::NSObjectProtocol;
use objc2_foundation::NSString;
use objc2_foundation::NSURL;
use objc2_foundation::NSURLRequest;
use objc2_foundation::NSURLResponse;

use objc2_web_kit::WKNavigation;
use objc2_web_kit::WKNavigationDelegate;
use objc2_web_kit::WKUIDelegate;
use objc2_web_kit::WKURLSchemeHandler;
use objc2_web_kit::WKURLSchemeTask;
use objc2_web_kit::WKUserScript;
use objc2_web_kit::WKUserScriptInjectionTime;
use objc2_web_kit::WKWebView;
use objc2_web_kit::WKWebViewConfiguration;

use raw_window_handle::RawWindowHandle;

use crate::ContextMenuItem;
use crate::CustomSchemeHandler;
use crate::WebViewError;

/// Flips `flag` on `didStartProvisionalNavigation:`/off on any of the three ways a main-frame
/// navigation can end (finish, or fail either before or after it committed). It is a plain instant
/// signal `WebViewInner::is_loading` polls, not a push callback, so `escher-bevy`/app code that
/// already ticks every frame (`sync_toolbar_state` in `apps/anvil`, say) can just read it alongside
/// everything else it's already re-checking, rather than needing new event plumbing threaded
/// through `escher-bevy`'s webview plugin. Exists because nothing anywhere in this app previously
/// showed *any* feedback while a page was loading. The toolbar just sat there looking identical
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

/// Forces the `WKWebView`'s own `NSAppearance` to Dark Aqua. See this module's doc comment for
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

/// Bridges one `ContextMenuItem::action` into AppKit's target-action mechanism, same shape as
/// `escher_appkit::action::ActionTarget`/`escher_os::macos::menu::MenuActionTarget`, duplicated
/// again rather than depended on (this crate has no dependency on either of those, and shouldn't
/// gain one just for this).
struct ContextMenuActionTargetIvars {
    action: Arc<dyn Fn() + Send + Sync>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `ContextMenuActionTarget` doesn't
    // implement `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ContextMenuActionTargetIvars]
    struct ContextMenuActionTarget;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for ContextMenuActionTarget {}

    impl ContextMenuActionTarget {
        // SAFETY: matches the `(id)sender` signature every `NSMenuItem` action selector is
        // invoked with. `sender` is never read, so its exact type doesn't matter here.
        #[unsafe(method(fire:))]
        fn fire(&self, _sender: &AnyObject) {
            (self.ivars().action)();
        }
    }
);

impl ContextMenuActionTarget {
    fn new(mtm: MainThreadMarker, action: Arc<dyn Fn() + Send + Sync>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ContextMenuActionTargetIvars { action });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}

/// Detects a right-click on a link and lets the caller (`on_link_context_menu`) append real,
/// working items to WebKit's own default menu (Open Link, Copy Link, ...). See
/// `WebView::attach`'s own doc comment for the contract.
///
/// Implements `webView:getContextMenuFromProposedMenu:forElement:completionHandler:`, the real,
/// macOS-only `WKUIDelegate` method for this (distinct from the iOS-only, UIKit-based
/// `contextMenuConfigurationForElement:`), as a plain selector, not through `WKUIDelegate`'s own
/// Rust trait: `objc2-web-kit` 0.3's generated binding for that trait doesn't declare this method
/// at all (confirmed by reading its generated source directly), and `WKContextMenuElementInfo`
/// itself has no bound properties either; its `linkURL` is read via a raw, unbound `msg_send!`
/// below. `define_class!` doesn't require a selector to be trait-declared to implement it (see
/// `ContextMenuActionTarget::fire:`/`escher_appkit::action::ActionTarget` for the same pattern).
/// WebKit calls optional delegate methods by checking `respondsToSelector:` at runtime, not
/// through any Rust-visible protocol conformance.
struct ContextMenuDelegateIvars {
    on_link_context_menu: Box<dyn Fn(&str) -> Vec<ContextMenuItem>>,
    /// Kept alive for exactly as long as the most recently shown menu might still need its
    /// targets. Replaced (dropping the previous menu's) each time a new context menu is
    /// requested, rather than accumulating forever.
    live_targets: RefCell<Vec<Retained<ContextMenuActionTarget>>>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `ContextMenuDelegate` doesn't implement
    // `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ContextMenuDelegateIvars]
    struct ContextMenuDelegate;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for ContextMenuDelegate {}

    // SAFETY: every `WKUIDelegate` method is `#[optional]`. Implementing none of them (the real
    // logic lives in the plain, non-trait `impl` block below) is a valid, empty conformance.
    unsafe impl WKUIDelegate for ContextMenuDelegate {}

    impl ContextMenuDelegate {
        // SAFETY: matches `webView:getContextMenuFromProposedMenu:forElement:completionHandler:`'s
        // real signature. `element`'s exact type is `WKContextMenuElementInfo`, guaranteed by
        // this delegate method's own contract (untyped here only because that class has no Rust
        // binding to type it as); `completion_handler` matches the `*mut NSMenu`-nullable-pointer
        // block convention `objc2-web-kit`'s other generated completion-handler methods already
        // use (e.g. `runOpenPanelWithParameters:...:completionHandler:`, which passes a nullable
        // `*mut NSArray<NSURL>` the same way).
        #[unsafe(method(webView:getContextMenuFromProposedMenu:forElement:completionHandler:))]
        fn get_context_menu(
            &self,
            _web_view: &WKWebView,
            proposed_menu: &NSMenu,
            element: &AnyObject,
            completion_handler: &block2::DynBlock<dyn Fn(*mut NSMenu)>,
        ) {
            // SAFETY: `linkURL` is a real `WKContextMenuElementInfo` property (`NSURL *`,
            // nullable), not bound in this crate version's generated stub for that class (it has
            // no bound properties at all), so read directly. `element` is guaranteed to actually
            // be a `WKContextMenuElementInfo` by this delegate method's own contract.
            let link_url: Option<Retained<NSURL>> = unsafe { msg_send![element, linkURL] };
            let link_url = link_url.and_then(|url| url.absoluteString()).map(|s| s.to_string());

            let Some(link_url) = link_url else {
                // Not a link. Leave WebKit's own proposed menu untouched.
                completion_handler.call((proposed_menu as *const NSMenu as *mut NSMenu,));
                return;
            };

            let extra_items = (self.ivars().on_link_context_menu)(&link_url);
            if extra_items.is_empty() {
                completion_handler.call((proposed_menu as *const NSMenu as *mut NSMenu,));
                return;
            }

            let mtm = MainThreadMarker::new().expect("WKUIDelegate callbacks run on the main thread");
            let mut targets = Vec::with_capacity(extra_items.len());

            proposed_menu.addItem(&NSMenuItem::separatorItem(mtm));
            for item in extra_items {
                let menu_item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), &NSString::from_str(&item.label), Some(sel!(fire:)), &NSString::from_str(""))
                };
                let target = ContextMenuActionTarget::new(mtm, item.action);
                // SAFETY: `target` is a real `ContextMenuActionTarget` responding to `fire:`
                // exactly as set above; kept alive in `live_targets` below for at least as long
                // as this menu is shown.
                unsafe { menu_item.setTarget(Some(&target)) };
                targets.push(target);
                proposed_menu.addItem(&menu_item);
            }

            *self.ivars().live_targets.borrow_mut() = targets;

            completion_handler.call((proposed_menu as *const NSMenu as *mut NSMenu,));
        }
    }
);

impl ContextMenuDelegate {
    fn new(mtm: MainThreadMarker, on_link_context_menu: impl Fn(&str) -> Vec<ContextMenuItem> + 'static) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ContextMenuDelegateIvars { on_link_context_menu: Box::new(on_link_context_menu), live_targets: RefCell::new(Vec::new()) });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}

/// Serves `CustomSchemeHandler::handler`'s answer for every request on its registered scheme,
/// fully, cleanly bound as a real `extern_protocol!` in `objc2-web-kit` (unlike the context-menu
/// delegate above, nothing here needed a raw `msg_send!` workaround). Synchronous: the handler
/// closure returns its whole HTML string in one call, so `webView:startURLSchemeTask:` answers
/// immediately (`didReceiveResponse:`/`didReceiveData:`/`didFinish` all called before returning).
/// There is no in-flight state for `webView:stopURLSchemeTask:` to actually cancel, so that method is a
/// no-op. Good enough for a static/computed-on-the-spot page (a settings page, say); a handler
/// that needs real async work (a network fetch) would need this reworked to hold the task and
/// answer later instead.
struct SchemeTaskHandlerIvars {
    handler: Arc<dyn Fn(&str) -> Option<crate::SchemeResponse> + Send + Sync>,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements; `SchemeTaskHandler` doesn't implement
    // `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SchemeTaskHandlerIvars]
    struct SchemeTaskHandler;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for SchemeTaskHandler {}

    // SAFETY: both methods match `WKURLSchemeHandler`'s real signatures exactly.
    unsafe impl WKURLSchemeHandler for SchemeTaskHandler {
        #[unsafe(method(webView:startURLSchemeTask:))]
        fn start_task(&self, _web_view: &WKWebView, url_scheme_task: &ProtocolObject<dyn WKURLSchemeTask>) {
            let request = unsafe { url_scheme_task.request() };
            let url = request.URL();
            let url_string = url.as_deref().and_then(|url| url.absoluteString()).map(|s| s.to_string()).unwrap_or_default();

            let Some(scheme_response) = (self.ivars().handler)(&url_string) else {
                let error = unsafe {
                    NSError::errorWithDomain_code_userInfo(&NSString::from_str("EscherWebViewCustomScheme"), 404, None)
                };
                unsafe { url_scheme_task.didFailWithError(&error) };
                return;
            };

            let Some(url) = url else {
                let error = unsafe { NSError::errorWithDomain_code_userInfo(&NSString::from_str("EscherWebViewCustomScheme"), 400, None) };
                unsafe { url_scheme_task.didFailWithError(&error) };
                return;
            };

            let bytes = scheme_response.body;
            let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
            let response = NSURLResponse::initWithURL_MIMEType_expectedContentLength_textEncodingName(
                NSURLResponse::alloc(),
                &url,
                Some(&NSString::from_str(&scheme_response.mime)),
                bytes.len() as isize,
                // Not every MIME type this now serves (images, fonts) is meaningfully "encoded
                // text" at all, but `WKURLSchemeTask` requires *some* value here; `None` would
                // read as "unspecified," not "binary," which isn't what's meant either. `utf-8`
                // is only actually load-bearing for the text-based MIME types (html/css/js/json).
                Some(&NSString::from_str("utf-8")),
            );

            unsafe {
                url_scheme_task.didReceiveResponse(&response);
                url_scheme_task.didReceiveData(&data);
                url_scheme_task.didFinish();
            }
        }

        #[unsafe(method(webView:stopURLSchemeTask:))]
        fn stop_task(&self, _web_view: &WKWebView, _url_scheme_task: &ProtocolObject<dyn WKURLSchemeTask>) {
            // No-op. See this struct's own doc comment: every task is already fully answered,
            // synchronously, before `webView:startURLSchemeTask:` even returns.
        }
    }
);

impl SchemeTaskHandler {
    fn new(mtm: MainThreadMarker, handler: Arc<dyn Fn(&str) -> Option<crate::SchemeResponse> + Send + Sync>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SchemeTaskHandlerIvars { handler });
        // SAFETY: `NSObject`'s `init` has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}

pub struct WebViewInner {
    handle: Retained<WKWebView>,
    /// Kept around (not just borrowed at `attach` time) so `set_left_inset` can re-read its
    /// *current* frame. Needed to recompute `handle`'s frame against the parent's live size
    /// rather than whatever size it happened to be at attach time.
    host: Retained<NSView>,
    top_inset: f64,
    loading: Arc<AtomicBool>,
    /// `setNavigationDelegate` is a *weak* property (see `objc2_web_kit`'s own doc comment on it)
    /// Nothing else keeps this alive, so without holding it here it would be deallocated the
    /// instant `attach` returns and no navigation callback would ever fire again.
    _navigation_delegate: Retained<NavigationDelegate>,
    /// `setUIDelegate` is weak too, same reasoning as `_navigation_delegate` above.
    _context_menu_delegate: Retained<ContextMenuDelegate>,
    /// `None` when `attach` wasn't given a `CustomSchemeHandler`, so there's nothing to keep alive.
    /// `setURLSchemeHandler:forURLScheme:` isn't documented as weak the way the two delegates
    /// above are, but held here anyway for the same "outlives what references it" safety margin.
    _scheme_task_handler: Option<Retained<SchemeTaskHandler>>,
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

    /// The loaded page's own `<title>`, read fresh each call rather than pushed via a delegate
    /// callback. `WKWebView::title` is a plain synchronous property (no async round-trip, unlike
    /// most of this API), so a caller already polling every tick (`is_loading` is read the exact
    /// same way) can just read this alongside it. `None` before any page has set one yet.
    pub fn title(&self) -> Option<String> {
        unsafe { self.handle.title() }.map(|title| title.to_string())
    }

    /// Despite the name (kept for API stability; every caller already thinks in terms of "show
    /// this tab's webview, hide the rest"), this no longer touches `NSView.isHidden` at all.
    /// `isHidden` is a well-documented rough edge on `WKWebView` specifically: toggling it back to
    /// `false` after being hidden triggers real internal work (re-establishing its compositor/
    /// backing store) that shows up as a visible stutter on every tab switch. Reordering
    /// z-position instead (`addSubview:positioned:relativeTo:`, moving the
    /// shown webview to the front of its superview's subviews and the hidden one to the back) gets
    /// the same visual result, exactly one webview actually on screen at a time, without ever
    /// touching the property that triggers that internal cost; macOS's own window-server occlusion
    /// tracking (separate from `isHidden`) still lets a fully-covered `WKWebView` throttle itself
    /// correctly, so this shouldn't cost more in the steady state either. `.Below`/`.Above` with no
    /// specific `relativeTo` view moves to the very back/front of all siblings, safe regardless of
    /// attach order, unlike relying on default z-order from whenever each tab happened to attach.
    pub fn set_hidden(&self, hidden: bool) {
        // SAFETY: `superview()` only reads the view hierarchy; `handle` is a real, live `NSView`
        // for as long as `self` exists.
        let Some(superview) = (unsafe { self.handle.superview() }) else { return };
        let place = if hidden { NSWindowOrderingMode::Below } else { NSWindowOrderingMode::Above };
        superview.addSubview_positioned_relativeTo(&self.handle, place, None);
    }

    /// Whether the main frame is currently between `didStartProvisionalNavigation:` and
    /// finishing/failing. See `NavigationDelegate`'s own doc comment.
    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Relaxed)
    }

    /// Injects `js` into every page this webview loads from now on (not the currently-loaded
    /// page retroactively — `WKUserScript`s only apply to navigations that happen after they're
    /// added). At document start, in every frame, in the page's own JS world (not an isolated
    /// one), so injected code can freely read/modify the page exactly like a real inline
    /// `<script>` would. This is the dev-tool "extension" mechanism (see `spec/.agents/
    /// proposals/webview-script-injection-mvp.md`): no `chrome.*`/`browser.*` API surface, no
    /// manifest-driven per-URL matching, just "run this JS on every page," the same as a
    /// userscript.
    pub fn add_script(&self, js: &str) {
        let mtm = MainThreadMarker::new().expect("add_script must be called from the main thread, same as every other WebView method");
        let script = unsafe {
            WKUserScript::initWithSource_injectionTime_forMainFrameOnly(
                WKUserScript::alloc(mtm),
                &NSString::from_str(js),
                WKUserScriptInjectionTime::AtDocumentStart,
                false,
            )
        };
        unsafe { self.handle.configuration().userContentController().addUserScript(&script) };
    }

    /// Re-trims the left edge (e.g. a tab strip being collapsed/expanded after this webview was
    /// already attached). Same inset math `attach` uses, just re-run against the host's current
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

impl Drop for WebViewInner {
    /// `attach` adds `handle` as a subview of the host window's content view (see `addSubview`
    /// below); nothing else ever removes it. Without this, closing a tab drops the Rust-side
    /// wrapper (`TabWebViews`'s `HashMap::remove`) but leaves the actual `WKWebView` still attached
    /// and visible, showing whatever it last rendered. Most visible in the last-tab-closed case,
    /// where nothing else covers it. This is also just honoring what `WebView`'s own doc
    /// comment already claims ("dropping it removes the webview from the window") but this
    /// backend never actually did.
    fn drop(&mut self) {
        self.handle.removeFromSuperview();
    }
}

pub fn attach(
    parent: RawWindowHandle,
    url: &str,
    top_inset: f64,
    left_inset: f64,
    user_agent: Option<&str>,
    on_link_context_menu: impl Fn(&str) -> Vec<ContextMenuItem> + 'static,
    custom_scheme: Option<CustomSchemeHandler>,
    initial_script: &str,
) -> Result<WebViewInner, WebViewError> {
    let RawWindowHandle::AppKit(appkit_handle) = parent else {
        return Err(WebViewError::UnsupportedWindowHandle);
    };

    let mtm = MainThreadMarker::new().ok_or(WebViewError::NotOnMainThread)?;

    // SAFETY: the caller is responsible for `parent` staying valid for as long as the returned
    // `WebView` is alive. Same contract `raw-window-handle` itself places on any
    // `RawWindowHandle` consumer.
    let ns_view: &NSView = unsafe { appkit_handle.ns_view.cast().as_ref() };

    // Starts `top_inset` points short of the parent's full height. Which end gets the `y` offset
    // depends on whether `parent` uses AppKit's default coordinate system (origin bottom-left, y
    // increasing upward, so leaving room at the top means trimming height, not offsetting `y`) or a
    // *flipped* one (origin top-left, y increasing downward, e.g. `bevy_winit`'s own content
    // view; leaving room at the top there means offsetting `y` *by* `top_inset`, not trimming from
    // the bottom). Getting this wrong doesn't error or misposition subtly. It silently reserves
    // the gap at the *wrong end* of the window, so the inset appears to do nothing at the top
    // (this exact bug, already found and fixed for the chrome bar's own `escher_os::macos::chrome
    // ::attach`, was never applied here, which is why the top inset stopped visibly working the
    // moment this got attached to a real `bevy_winit` window instead of the non-flipped `NSWindow`
    // this was presumably last visually checked against).
    //
    // Paired with the `ViewHeightSizable` mask below (not `ViewMinYMargin`/`ViewMaxYMargin`, which
    // would instead keep this fixed-size and let the *gap* grow): as the window resizes, this
    // view's own height changes to absorb the difference while both margins, the `top_inset`-tall
    // gap and its flush-zero gap on the other side, stay constant, regardless of which one is
    // geometrically "at the top" for this particular parent.
    // `left_inset` (e.g. reserving space for a tab strip) is unaffected by flippedness: x
    // increases rightward regardless, so it's just a plain trim off the left edge, paired with
    // `ViewWidthSizable` the same way `top_inset` is paired with `ViewHeightSizable` below.
    let parent_frame = ns_view.frame();
    let is_flipped = ns_view.isFlipped();
    let inset_frame = objc2_foundation::NSRect {
        origin: objc2_foundation::NSPoint { x: left_inset, y: if is_flipped { top_inset } else { 0.0 } },
        size: objc2_foundation::NSSize { width: parent_frame.size.width - left_inset, height: parent_frame.size.height - top_inset },
    };

    let configuration = unsafe { WKWebViewConfiguration::new(mtm) };

    // Must happen before `initWithFrame:configuration:` below. `WKWebViewConfiguration` is
    // snapshotted at webview-init time, so registering a scheme handler on it afterward would
    // silently do nothing.
    let scheme_task_handler = custom_scheme.map(|custom_scheme| {
        let handler = SchemeTaskHandler::new(mtm, custom_scheme.handler);
        unsafe {
            configuration.setURLSchemeHandler_forURLScheme(Some(ProtocolObject::from_ref(&*handler)), &NSString::from_str(&custom_scheme.scheme))
        };
        handler
    });

    let webview = unsafe { WKWebView::initWithFrame_configuration(mtm.alloc(), inset_frame, &configuration) };

    webview.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);

    // `WKWebView.isInspectable` (macOS 13.3+) is not yet in `objc2-web-kit` 0.3.2's bindings, same
    // "no bound property, raw `msg_send!` instead" situation `ContextMenuDelegate`'s own `linkURL`
    // read is in below. Lets every webview this app creates show up under Safari's own Develop
    // menu (Develop ▸ <this process> ▸ <page title>) with a real Web Inspector attached. Anvil is
    // a developer tool for its own users, not something shipped to an audience that shouldn't see
    // this, so there's no reason to gate it behind a debug-only build.
    let _: () = unsafe { msg_send![&webview, setInspectable: true] };

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

    let context_menu_delegate = ContextMenuDelegate::new(mtm, on_link_context_menu);
    unsafe { webview.setUIDelegate(Some(ProtocolObject::from_ref(&*context_menu_delegate))) };

    let inner = WebViewInner {
        handle: webview,
        host: ns_view.retain(),
        top_inset,
        loading,
        _navigation_delegate: navigation_delegate,
        _context_menu_delegate: context_menu_delegate,
        _scheme_task_handler: scheme_task_handler,
    };
    // Registered before the initial `load` below: `WKUserScript`s only apply to navigations that
    // start after they're added, so this order is what makes the very first page this webview
    // ever shows get `initial_script` too, not just every one after it.
    if !initial_script.is_empty() {
        inner.add_script(initial_script);
    }
    inner.load(url)?;

    ns_view.addSubview(&inner.handle);

    Ok(inner)
}
