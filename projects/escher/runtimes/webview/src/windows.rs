//! Windows backend: attaches a WebView2-hosted view to an `HWND` via the `webview2` crate (a
//! callback-based wrapper around the real `ICoreWebView2`/`ICoreWebView2Controller` COM
//! interfaces). It's the same crate and the same API shape `runtimes/bevy/src/legacy/webview.rs`
//! already used against a pre-refactor prototype, ported here against the current
//! `escher-webview` contract instead of hand-rolling raw COM bindings from scratch (lower risk:
//! this exact crate/API is proven to work against a real WebView2 install, unlike code derived
//! fresh against the raw COM interfaces with no way to build+run it here to check).
//!
//! **Real, load-bearing difference from the macOS backend, unlike everything else here**:
//! `WKWebView::initWithFrame_configuration` creates a fully-usable webview synchronously, so
//! `macos::attach` can build one, attach it, and hand back a ready `WebViewInner` in one call.
//! WebView2's own environment/controller creation is inherently asynchronous (a real round trip
//! through the browser process). `EnvironmentBuilder::build`/`Environment::create_controller`
//! both take a completion callback invoked *later*, once the OS message loop this thread is
//! already running (Bevy/winit's own event loop, ticking every frame regardless) happens to
//! deliver it. That's typically within a frame or two of `attach` returning, never synchronously
//! within it. So `attach` here kicks off that async chain and returns immediately with a
//! `WebViewInner` whose `ready` cell starts `None`; the completion callback (still running on
//! this same thread, since nothing here is genuinely multi-threaded) populates it once the real
//! `Controller`/`WebView` exist. Every other method below is written to degrade gracefully
//! (no-op / return `false`) during that narrow startup window rather than assume `ready` is
//! always populated.
//!
//! **Known gap, not yet closed**: unlike `WKWebView`'s autoresizing mask, `ICoreWebView2Controller`
//! has no "keep filling my parent" behavior of its own. Its `Bounds` needs updating on every
//! parent-window resize or the webview visibly stays whatever size it was at attach time. Closed
//! here via a `WM_SIZE` window-procedure subclass on the parent `HWND` (see `subclass_for_resize`)
//! rather than left as a silent gap, but this is the one piece of this file most likely to need a
//! real fix once actually run on Windows. It's the only part with no equivalent already proven
//! to work in the legacy prototype this was ported from.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use webview2::{Controller, Environment, EnvironmentBuilder, WebResourceContext, WebView as CoreWebView};

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::winuser::{GetClientRect, SetWindowLongPtrW, CallWindowProcW, GWLP_WNDPROC, WM_SIZE};

use raw_window_handle::RawWindowHandle;

use crate::ContextMenuItem;
use crate::CustomSchemeHandler;
use crate::WebViewError;

/// Populated once the async environment+controller creation chain `attach` kicks off actually
/// finishes. See this module's own doc comment.
struct Ready {
    /// Never read again after creation. Its only job is keeping the WebView2 environment alive
    /// for as long as `controller`/`webview` (both created from it) need it to be.
    #[allow(dead_code)]
    environment: Environment,
    controller: Controller,
    webview: CoreWebView,
}

pub struct WebViewInner {
    hwnd: HWND,
    top_inset: f64,
    left_inset: Rc<RefCell<f64>>,
    hidden: Rc<RefCell<bool>>,
    loading: Arc<AtomicBool>,
    ready: Rc<RefCell<Option<Ready>>>,
    /// `load()` called before `ready` is populated remembers its most recent request here instead
    /// of dropping it. It's applied the moment the completion callback populates `ready` (see
    /// `attach`'s own closure).
    pending_url: Rc<RefCell<Option<String>>>,
}

fn current_bounds(hwnd: HWND, top_inset: f64, left_inset: f64) -> RECT {
    // SAFETY: `hwnd` is a live window handle for as long as `WebViewInner` is (the same contract
    // every `RawWindowHandle` consumer already places on its caller); `client_rect` is a plain
    // stack `RECT` `GetClientRect` writes into, matching its documented signature exactly.
    let mut client_rect: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetClientRect(hwnd, &mut client_rect) };
    RECT { left: left_inset as i32, top: top_inset as i32, right: client_rect.right, bottom: client_rect.bottom }
}

impl WebViewInner {
    pub fn load(&self, url: &str) -> Result<(), WebViewError> {
        match self.ready.borrow().as_ref() {
            Some(ready) => {
                if let Err(error) = ready.webview.navigate(url) {
                    tracing::warn!("WebView2 navigate failed: {error}");
                    return Err(WebViewError::InvalidUrl);
                }
                tracing::info!("WebView loading {url}");
            }
            // Not ready yet. Remembered and applied the instant the completion callback in
            // `attach` populates `ready` (see its own doc comment on why this window is narrow
            // but real).
            None => *self.pending_url.borrow_mut() = Some(url.to_string()),
        }
        Ok(())
    }

    pub fn go_back(&self) {
        if let Some(ready) = self.ready.borrow().as_ref()
            && let Err(error) = ready.webview.go_back()
        {
            tracing::warn!("WebView2 go_back failed: {error}");
        }
    }

    pub fn go_forward(&self) {
        if let Some(ready) = self.ready.borrow().as_ref()
            && let Err(error) = ready.webview.go_forward()
        {
            tracing::warn!("WebView2 go_forward failed: {error}");
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.ready.borrow().as_ref().and_then(|ready| ready.webview.get_can_go_back().ok()).unwrap_or(false)
    }

    pub fn can_go_forward(&self) -> bool {
        self.ready.borrow().as_ref().and_then(|ready| ready.webview.get_can_go_forward().ok()).unwrap_or(false)
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Relaxed)
    }

    /// The loaded page's own document title. `None` before `ready` is populated or before any
    /// page has set one yet, same "not ready" degradation every other method here already uses.
    pub fn title(&self) -> Option<String> {
        self.ready.borrow().as_ref().and_then(|ready| ready.webview.get_document_title().ok())
    }

    /// Unlike the macOS backend's `set_hidden` (which reorders z-position specifically to dodge a
    /// `WKWebView`-only recompositing stutter on `isHidden`; see its own doc comment), this just
    /// toggles `ICoreWebView2Controller::put_is_visible` directly; no equivalent stutter is
    /// documented for WebView2, and there's no cheaper alternative available through this crate's
    /// API anyway.
    pub fn set_hidden(&self, hidden: bool) {
        *self.hidden.borrow_mut() = hidden;
        if let Some(ready) = self.ready.borrow().as_ref()
            && let Err(error) = ready.controller.put_is_visible(!hidden)
        {
            tracing::warn!("WebView2 put_is_visible failed: {error}");
        }
    }

    pub fn set_left_inset(&self, left_inset: f64) {
        *self.left_inset.borrow_mut() = left_inset;
        if let Some(ready) = self.ready.borrow().as_ref() {
            let bounds = current_bounds(self.hwnd, self.top_inset, left_inset);
            if let Err(error) = ready.controller.put_bounds(bounds) {
                tracing::warn!("WebView2 put_bounds failed: {error}");
            }
        }
    }
}

impl Drop for WebViewInner {
    /// `Controller::close` is WebView2's own documented teardown call (tears down the browser
    /// process's side of this controller/webview pair). It's the real equivalent of the macOS
    /// backend's `removeFromSuperview()`, not just dropping the Rust-side handle and hoping.
    fn drop(&mut self) {
        if let Some(ready) = self.ready.borrow_mut().take()
            && let Err(error) = ready.controller.close()
        {
            tracing::warn!("WebView2 controller close failed: {error}");
        }
    }
}

/// Subclasses `hwnd`'s window procedure so `controller`'s bounds track every `WM_SIZE`. See this
/// module's own doc comment on why WebView2, unlike `WKWebView`, needs this instead of an
/// autoresizing mask. Stashes the *original* window procedure in `hwnd`'s user data slot
/// (`GWLP_USERDATA`) rather than a global/thread-local map, so this stays correct even if more
/// than one such window exists in the same process. Each subclassed `HWND` carries its own
/// original-procedure pointer right on itself.
fn subclass_for_resize(hwnd: HWND, top_inset: f64, left_inset: Rc<RefCell<f64>>, controller: Controller) {
    use std::sync::Mutex;
    use winapi::shared::basetsd::LONG_PTR;
    use winapi::um::winuser::{GetWindowLongPtrW, GWLP_USERDATA};

    struct ResizeState {
        original_proc: LONG_PTR,
        top_inset: f64,
        left_inset: Rc<RefCell<f64>>,
        controller: Controller,
    }

    // SAFETY: only ever touched from this process's single UI thread. Matches every other
    // COM/HWND call in this module, none of which are genuinely cross-thread.
    thread_local! {
        static STATES: Mutex<Vec<Box<ResizeState>>> = Mutex::new(Vec::new());
    }

    // SAFETY: matches `WNDPROC`'s real signature exactly; only ever installed via
    // `SetWindowLongPtrW` below, on a window that's guaranteed to carry a `ResizeState` in its
    // `GWLP_USERDATA` slot (set immediately before installing this).
    unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const ResizeState;
        let state = unsafe { &*state_ptr };

        if msg == WM_SIZE {
            let bounds = current_bounds(hwnd, state.top_inset, *state.left_inset.borrow());
            if let Err(error) = state.controller.put_bounds(bounds) {
                tracing::warn!("WebView2 put_bounds on resize failed: {error}");
            }
        }

        // SAFETY: `original_proc` was itself the real return value of a prior `SetWindowLongPtrW(
        // ..., GWLP_WNDPROC, ...)` call. It's a valid `WNDPROC`-typed value reinterpreted as
        // `LONG_PTR` to store, transmuted back to its real type here. Same pointer-sized
        // reinterpretation every raw Win32 subclassing helper does; `as` alone can't perform it
        // (an optional extern-fn-pointer isn't a primitive cast target).
        let original_proc: winapi::um::winuser::WNDPROC = unsafe { std::mem::transmute(state.original_proc) };
        unsafe { CallWindowProcW(original_proc, hwnd, msg, wparam, lparam) }
    }

    let state = Box::new(ResizeState { original_proc: 0, top_inset, left_inset, controller });
    let state_ptr = Box::into_raw(state);

    // SAFETY: `hwnd` is a live window for as long as this process's webview is; `GWLP_USERDATA`
    // is ours to use freely (this crate is the only thing subclassing this window).
    unsafe { winapi::um::winuser::SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as LONG_PTR) };
    let original_proc = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, subclass_proc as *const () as LONG_PTR) };
    unsafe { (*state_ptr).original_proc = original_proc };

    // Leaked deliberately: this `HWND` lives for the lifetime of the whole browser window, and
    // nothing here ever un-subclasses it. Same "outlives what references it, no explicit
    // teardown path exists yet" tradeoff `WebViewInner`'s own delegate fields on macOS document
    // for weak AppKit properties, just via a raw allocation instead of a `Retained<T>` here.
    STATES.with(|states| states.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(unsafe { Box::from_raw(state_ptr) }));
}

pub fn attach(
    parent: RawWindowHandle,
    url: &str,
    top_inset: f64,
    left_inset: f64,
    _user_agent: Option<&str>,
    _on_link_context_menu: impl Fn(&str) -> Vec<ContextMenuItem> + 'static,
    custom_scheme: Option<CustomSchemeHandler>,
) -> Result<WebViewInner, WebViewError> {
    let RawWindowHandle::Win32(win32_handle) = parent else {
        return Err(WebViewError::UnsupportedWindowHandle);
    };
    let hwnd = win32_handle.hwnd.get() as HWND;

    let url = url.to_string();
    let loading = Arc::new(AtomicBool::new(false));
    let ready: Rc<RefCell<Option<Ready>>> = Rc::new(RefCell::new(None));
    let pending_url: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let left_inset_cell = Rc::new(RefCell::new(left_inset));
    let hidden_cell = Rc::new(RefCell::new(false));

    // Every one of these is also read from `WebViewInner` below (constructed after this whole
    // async chain has already been *kicked off*, well before it *finishes*; see this module's
    // own doc comment). Cloned here, not moved, so both the completion closure and the
    // `WebViewInner` this function returns end up sharing the same cells rather than the closure
    // taking the only copies.
    let build_result = {
        let ready = ready.clone();
        let pending_url = pending_url.clone();
        let left_inset_cell = left_inset_cell.clone();
        let hidden_cell = hidden_cell.clone();
        let loading = loading.clone();

        EnvironmentBuilder::new().build(move |environment| {
            let environment = environment?;

            // Cloned *before* the `create_controller` call below, not inside its closure.
            // `environment.create_controller(...)` borrows `environment` for the receiver at the
            // same time a `move` closure argument would otherwise need to move it whole to use
            // (even just to `.clone()` it, or to move it into `Ready` below), which conflicts
            // (can't borrow and move the same variable within one expression). Two clones, not
            // one shared between both uses: `environment_for_handler` is captured into the
            // *inner* (`add_web_resource_requested`) closure, called repeatedly for the webview's
            // whole lifetime; `environment_for_ready` is captured into this *outer* completion
            // closure just once, to end up owned by `Ready`.
            let environment_for_handler = environment.clone();
            let environment_for_ready = environment.clone();

            environment.create_controller(hwnd, move |controller| {
                let controller = controller?;
                let webview = controller.get_webview()?;

                let started = loading.clone();
                webview.add_navigation_starting(move |_, _| {
                    started.store(true, Ordering::Relaxed);
                    Ok(())
                })?;
                let finished = loading.clone();
                webview.add_navigation_completed(move |_, _| {
                    finished.store(false, Ordering::Relaxed);
                    Ok(())
                })?;

                if let Some(custom_scheme) = custom_scheme {
                    let filter = format!("{}://*", custom_scheme.scheme);
                    webview.add_web_resource_requested_filter(&filter, WebResourceContext::All)?;
                    webview.add_web_resource_requested(move |_, args| {
                        let request = args.get_request()?;
                        let requested_url = request.get_uri()?;
                        let (status, reason, body) = match (custom_scheme.handler)(&requested_url) {
                            Some(html) => (200, "OK", html),
                            None => (404, "Not Found", String::new()),
                        };
                        let stream = webview2::Stream::from_bytes(body.as_bytes());
                        let response = environment_for_handler.create_web_resource_response(stream, status, reason, "Content-Type: text/html")?;
                        args.put_response(response)?;
                        Ok(())
                    })?;
                }

                controller.put_bounds(current_bounds(hwnd, top_inset, *left_inset_cell.borrow()))?;
                controller.put_is_visible(!*hidden_cell.borrow())?;

                subclass_for_resize(hwnd, top_inset, left_inset_cell.clone(), controller.clone());

                let initial_url = pending_url.borrow_mut().take().unwrap_or(url);
                webview.navigate(&initial_url)?;

                *ready.borrow_mut() = Some(Ready { environment: environment_for_ready, controller, webview });
                Ok(())
            })
        })
    };

    if let Err(error) = build_result {
        return Err(WebViewError::PlatformError(error.to_string()));
    }

    Ok(WebViewInner { hwnd, top_inset, left_inset: left_inset_cell, hidden: hidden_cell, loading, ready, pending_url })
}
