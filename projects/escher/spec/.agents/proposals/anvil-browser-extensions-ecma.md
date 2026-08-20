# Proposal: browser extensions for Anvil, built on Ethos's ECMA runtime

Status: **not implemented — proposal only**, per `AGENTS.md`'s default. Written up from a scoping
investigation on 2026-08-15 (human asked "how close are we to extensions using the built-in Deno
scripting engine," then redirected mid-investigation to "explore viability of using the Ethos ecma
embeddable scripting language" specifically). No code written for this yet.

## The ask

Build an extension system for Anvil's browser (the AppKit toolbar/tab-strip + WKWebView surface),
scriptable in JS/TS, built on Ethos's existing embeddable scripting stack rather than starting a
JS-engine integration from scratch.

## Current state (verified via search, 2026-08-15)

**The engine half already exists and works — it's just never been connected to Anvil.**

- `ethos-ecma` (`ethos/dialects/ecma`) — parsing/printing only, via `swc_core`. Explicitly no
  execution (its own `Cargo.toml` comment: *"No execution — that's `ethos-deno`'s job"*).
- `ethos-deno` (`ethos/packages/deno`) — the real execution half. Genuine `deno_core` +
  `deno_runtime` dependencies (not just a name), built as `crate-type = ["rlib", "cdylib"]` with a
  `feature = "ffi"` exposing a C-ABI for native hosts to call into. This is what backs Aby's Unity
  integration today: `runtimes/unity/Assets/Plugins/Escher/{EcmaRuntime.g.cs,libecma.dylib}` is a
  live, working, currently-shipping instance of this exact runtime, driving Aby's game scripting.
- The two crates are deliberately decoupled (no Cargo dependency between them), paired only at the
  CLI level by `ethos-cli`.
- **Confirmed zero connection to Escher/Anvil**: no `deno_core`/`deno_runtime`/`rusty_v8` anywhere
  in the Escher workspace's `Cargo.lock`; no reference to `libecma`/`EcmaRuntime`/Deno anywhere in
  `apps/anvil`, `runtimes/appkit`, or `runtimes/webview`.
- **`escher-webview` has no content-script injection today** — no `WKUserScript`, no
  `evaluateJavaScript`, no `addUserScript` anywhere in `runtimes/webview/src/{lib,macos}.rs`. Pages
  render; nothing can run script into or read out of them yet.
- **Caveat, not chased further**: `aby/scripts/build-ecma.sh` references paths (`../packages/ecma`,
  `../runtimes/Unity`) that don't match Ethos's current layout (`ethos/dialects/ecma`,
  `ethos/packages/deno`) — the script may be stale. Worth a real build attempt before trusting it
  as documentation of "how the Aby integration currently builds."

## What's missing (the actual scope of this proposal)

The hard part — an embeddable JS/TS engine with a proven native-host integration — already exists.
What doesn't exist, and is genuinely new work:

1. **A browser-shaped host API surface.** Aby's existing FFI bindings are game-scripting-shaped
   (Unity object/component access) — none of it is reusable for "list tabs," "navigate," "read
   page content," etc. This is the actual design work: deciding what an extension can call and
   what it looks like from the JS side.
2. **Content-script injection into WKWebView.** Doesn't exist at all in `escher-webview` — needs
   `WKUserScript`/`evaluateJavaScript` plumbing added before an extension can touch page content.
3. **A permission/sandboxing policy.** `deno_core` supports fine-grained permission scoping
   natively (its whole design point) — but no policy exists yet for what an Anvil extension should
   be allowed to touch (network? filesystem? which host APIs?).
4. **An extension manifest/loader format**, and wiring a `MainWorker`-per-extension lifecycle into
   Anvil's Bevy `Update` schedule (same shape of integration `ToolbarPlugin`/`WebViewPlugin` already
   do for their own native surfaces — a plugin owning non-Send resources, attach/detach lifecycle).
5. **Open question, not resolved here**: whether to consume `ethos-deno` through its existing
   C-ABI/`cdylib` boundary (matches how Aby/Unity already does it, but adds an FFI hop) or link it
   as a normal Rust dependency directly into `apps/anvil` (cheaper, no FFI boundary, but means
   `ethos-deno`'s `ffi` feature/C-ABI work isn't what's being reused — only the Rust API under it).
   This changes the shape of the integration meaningfully and should be answered before starting.

## Explicitly out of scope for this proposal

- Extension *distribution* (a store, signing, update mechanism) — irrelevant until a single
  extension can run at all.
- Cross-runtime extension support (Hudd/Desktop) — Anvil only, first.
- Anything about `ethos-ecma`'s parse/print half changing — this proposal only concerns consuming
  `ethos-deno`'s execution runtime; no dialect-level work is implied.

## Suggested shortest path to a viability spike (not a commitment, just the cheapest next step)

Link `ethos-deno` directly as a Rust dependency into a throwaway example inside `apps/anvil` (or a
new `experiments/` sandbox), skip the FFI/C-ABI boundary entirely, and get a `MainWorker` executing
one trivial script that calls back into one stubbed host function (e.g. "print the active tab's
title"). That single spike answers the open question in item 5 above empirically and would be the
natural first step whenever this is picked up.
