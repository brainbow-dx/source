# Full extension support (deferred)

Status: not started, deliberately deferred. `webview-script-injection-mvp.md` covers what's
actually built today — a plain JS/CSS injection mechanism, enough to validate a content script's
DOM/CSS manipulation logic, nothing that touches `chrome.*`/`browser.*` APIs. This doc is the
scoped-out "what would real extension support actually take" answer, written up so it isn't lost
and doesn't need re-deriving from scratch later.

## The real finding: this is more reachable than general knowledge suggests

Going in, the assumption was that `WKWebView` has no path to hosting real extensions at all short
of reimplementing a `chrome.*` shim and a manifest engine from scratch (the way Orion/Arc had to).
That assumption is **wrong as of a recent WebKit version**. `objc2-web-kit` (already a dependency
of `escher-webview`, no new crate needed) ships real bindings for:

- `WKWebExtension` — loads a real, on-disk WebExtension (Manifest V2/V3-shaped: `manifest.json`,
  content scripts, a background service worker/page, `chrome.*`/`browser.*` API surface) as a
  first-class object.
- `WKWebExtensionContext` — one loaded extension's runtime state: permissions (grant/deny per
  `WKWebExtensionPermission`, per `WKWebExtensionMatchPattern`), errors, tabs, windows.
- `WKWebExtensionController` (+ `WKWebExtensionControllerConfiguration`) — the thing that actually
  wires a loaded extension's content scripts into real `WKWebView`s and dispatches its background
  logic, analogous to a browser's own extension host process.
- `WKWebExtensionTab`/`WKWebExtensionWindow` (+ their `...Configuration` types) — protocols this
  app would implement so the extension's own `chrome.tabs`/`chrome.windows` calls resolve against
  Anvil's actual tab strip instead of nothing.

All of the above are already in `objc2-web-kit`'s **default** feature set (confirmed by reading
its `Cargo.toml` directly, not assumed) — `escher-webview` already pulls them in with zero
Cargo.toml changes. This is Apple's own native answer to "let a `WKWebView`-based app host real
extensions," not something bolted on by a third party. This changes the shape of "full support"
from "reimplement a browser extension host from scratch" to "bind a real Apple API and implement a
handful of protocol conformances" — still real, non-trivial work, but a fundamentally different
scale of effort than the pre-research assumption.

**One real unknown, not yet resolved**: the minimum macOS/WebKit version this API requires. It's
recent — recent enough that it wasn't in general pre-training knowledge — and this session had no
way to confirm the exact version number from local sources alone. Before committing real
implementation time to this path, confirm the actual `#[cfg]`/availability requirements against
Apple's real documentation (not guessed), and check what macOS version this development machine
(and any real target audience) is actually running. If the requirement is newer than what's
realistic to require, the MVP's plain injection mechanism is still the fallback, indefinitely.

## What real support on top of `WKWebExtension` would take

Rough shape, not a committed plan:

1. **Loading**: point `WKWebExtensionController` at an unpacked extension directory (a real
   `manifest.json` + its files) instead of `webview-script-injection-mvp.md`'s flat `.js`/`.css`
   directory convention. `.anvil.toml`'s `extensions` list could plausibly stay the same shape
   (a list of directories), just interpreted differently once a `manifest.json` is present in one.
2. **Tab/window protocol conformances**: implement `WKWebExtensionTab`/`WKWebExtensionWindow` (and
   their delegate methods) against Anvil's real `BrowserState`/tab strip, so `chrome.tabs.query`
   etc. return real data instead of nothing. This is almost certainly the largest single piece of
   work — it's where "a real extension host" and "Anvil's own tab model" actually have to talk to
   each other.
3. **Permissions**: a real UI for granting/denying `WKWebExtensionPermission`/
   `WKWebExtensionMatchPattern` requests, even a minimal one (a modal listing what the extension
   asked for, allow/deny). `WKWebExtensionContext` exposes the request surface; this app has no UI
   for it yet.
4. **Background content**: `WKWebExtensionContext` reports whether background content
   loaded/failed (`WKWebExtensionContextErrorNoBackgroundContent`/
   `BackgroundContentFailedToLoad`) — real error handling needed here, not just logging and
   moving on the way most of this codebase's "best-effort, degrade gracefully" paths do, since a
   silently-failed background script is exactly the kind of thing an extension developer is trying
   to catch by testing here in the first place.
5. **Windows**: WebView2 has its own real (Microsoft-labeled experimental) path,
   `ICoreWebView2Profile::AddBrowserExtension`, for loading actual unpacked Chrome/Edge extensions
   — genuinely real Manifest V3 compatibility, not a shim, on Windows specifically. Not
   investigated in this session at the same depth as the macOS `WKWebExtension` finding above;
   worth its own real look before assuming parity is or isn't reachable there.

## Explicit non-goals, even for "full" support

- **Safari-specific extension distribution/signing** (the App Store review flow real Safari Web
  Extensions go through) — irrelevant here; this is a dev tool loading unpacked/local extensions,
  not a channel for publishing them.
- **Chrome Web Store installation** — no path to installing directly from the Chrome Web Store's
  own `.crx` format is implied by anything above; `WKWebExtension` loads from a local directory,
  same as Chrome's own "load unpacked" developer mode.
- **Perfect `chrome.*` API parity** — `WKWebExtension`'s own API coverage is real but not
  necessarily 100% of what Chrome/Edge expose; some extensions may still hit gaps. Not something
  to chase exhaustively; the goal is "verify what should work does," not "guarantee every
  extension in existence works unmodified."

## Why this stays deferred for now

Per direct user framing: this is a developer tool, not a daily-driver browser, and the actual
near-term want is narrower — install and verify a real extension's content-script/CSS logic during
development, which `webview-script-injection-mvp.md`'s plain injection mechanism already covers
for the common case. The `WKWebExtension` path above is real and worth doing eventually, but it's
a multi-session effort (tab/window protocol conformances especially), not something to start
without a concrete near-term need driving it.
