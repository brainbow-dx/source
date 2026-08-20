# Webview script injection (dev-tool "extension" support MVP)

Prompted by a direct ask: "gut check" whether basic Chrome/Edge/Safari extension support was
reachable in `escher-webview`. Full extension compatibility (`chrome.*`/`browser.*` APIs, a
manifest-driven permission/matching model) was explicitly scoped out for now — this is a
developer tool, not a daily-driver browser, and the actual want is narrower: install and verify a
real extension's content-script/CSS logic during development. See `webview-extension-support-
full.md` for the deferred bigger scope and why it's more reachable than it first looked.

## Status: implemented

The mechanism: `WebView::add_script(js)` and a new `initial_script` parameter on
`WebView::attach`, backed by each platform's own native "run this JS on every page load" API —
`WKUserContentController.addUserScript` on macOS, `AddScriptToExecuteOnDocumentCreated` on
Windows (WebView2). No `chrome.*`/`browser.*` shim, no manifest-driven per-URL matching, no
isolated execution world: injected code runs in the page's own JS world at document-start, on
every frame, the same as a plain userscript. That's a deliberate scope cut, not an oversight — see
the full-support proposal for what layering matching/isolation/an API surface on top would take.

- `escher-webview` (`runtimes/webview/src/{lib,macos,windows}.rs`):
  - `WebView::add_script(&self, js: &str)` — adds a script that applies to every navigation *from
    this call on*, not retroactively to whatever's currently loaded. macOS: a `WKUserScript`
    added to the webview's own `WKUserContentController`. Windows: `ICoreWebView2::
    add_script_to_execute_on_document_created`, queued in a new `pending_scripts` cell and applied
    once the async controller-creation chain (`attach`'s own doc comment explains why that's
    real, not theoretical) finishes if called before then.
  - `WebView::attach` gained a new `initial_script: &str` parameter. Real bug this closes: `attach`
    already kicks off the webview's first navigation before returning (loading `url` is what
    `attach` *does*), so a script only added via `add_script` afterward would miss that very first
    page load — `WKUserScript`s and WebView2's document-created scripts only apply to navigations
    that start after they're registered. `initial_script` is registered before that first
    navigation instead, both on macOS (`inner.add_script` called before `inner.load(url)`) and
    Windows (`pending_scripts` seeded up front, drained before the first `navigate` call inside the
    completion closure). Empty string is a harmless no-op, the common case (no extensions
    configured).
  - `runtimes/bevy/src/webview.rs`'s `WantsWebView` component gained a matching `initial_script`
    field for API completeness, though nothing constructs one yet (this plugin has no live caller
    today — Anvil calls `escher_webview::WebView::attach` directly, not through this component).

- `apps/anvil`:
  - `.anvil.toml` gained an `extensions: Option<Vec<String>>` field — a list of directories
    (relative to the project directory, same convention as everything else in that file), each
    holding any number of `.js`/`.css` files.
  - New `extensions.rs`: `load_extensions(dirs)` reads every `.js`/`.css` file directly inside
    each directory (non-recursive, no `manifest.json`) and combines them into one script.
    `.js` files are concatenated as-is; `.css` files are wrapped in a small JS snippet that
    appends a `<style>` element (there's no separate native "inject CSS" call on either backend —
    layering it on the one JS-injection mechanism keeps this to one code path instead of two).
    Content is JSON-encoded before being dropped into that snippet so a stylesheet containing a
    literal backtick or `</script>` can't break out of it.
  - Resolved once in `main`, stored on `AppState::extensions_script`, threaded into `WebView::
    attach`'s new `initial_script` parameter at `attach_pending_tab_webviews` — every browser tab
    gets it, not just the first one opened.

Real, non-obvious finding along the way: `objc2-web-kit` (already a dependency here) also has
bindings for `WKWebExtensionContext`/`WKWebExtensionController` — Apple's own, genuinely real
WebExtension-format hosting API, added to WebKit fairly recently. That's a materially different
(and better) starting point for real extension support than "WKWebView has nothing," which is
what general knowledge would suggest without actually checking. See `webview-extension-support-
full.md`.

## Known limits, by design, not bugs

- No per-URL matching (a manifest's `content_scripts[].matches`) — every script/style runs on
  every page, unconditionally. A real extension author testing something URL-scoped needs to
  guard that themselves in their own script for now.
- No `chrome.*`/`browser.*` API surface at all. Any content script that references
  `chrome.runtime`/`chrome.storage`/etc. throws immediately (`chrome is not defined`) — this only
  validates DOM/CSS manipulation logic, not anything touching extension messaging or storage.
- Files within one extension directory load in directory-listing order, not guaranteed consistent
  across platforms/filesystems. Fine for independent scripts; an extension needing a specific
  order between its own files should combine them into one file itself for now.
- No live-reload — `.anvil.toml`'s `extensions` list is read once at startup. Changing a script
  needs an Anvil restart to pick up.
- Windows path (`windows.rs`) is implemented but **unverified** — no Windows toolchain available
  to build/run it in this environment. The macOS path (what this session's actual verification
  covered: `cargo check` clean on `escher-webview`, `escher-bevy`, `escher-anvil`) is the only one
  actually exercised.

## Follow-ups not done here

- Live-test on a real macOS machine: open a tab, confirm an injected script/style actually applies
  to the very first page load, not just subsequent navigations (this is exactly the bug
  `initial_script` was added to fix — worth confirming it actually works, not just that it
  compiles).
- Verify the Windows path on real hardware once that's feasible.
- Everything in `webview-extension-support-full.md`.
