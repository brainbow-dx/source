# Legacy reference (not compiled)

Ported from `/Volumes/Bob/PC Backup - March 2026/Open/projects/slate/runtimes/bevy` (the
pre-refactor "slate"-era version of this project), 2026-08-13. Kept for reference; none of these
files are declared as modules in `src/lib.rs`, so they don't build as part of this crate.

- **`window.rs`, `webview.rs`, `input.rs`** — hard-locked to Windows: they import `webview2`
  (wraps the Edge WebView2 COM control) and `winapi::um::winuser` directly and unconditionally.
  The old `Cargo.toml` had `cocoa`/`objc` listed under `cfg(target_os = "macos")`, but no macOS
  implementation actually exists in these files — that work was never finished. Getting a
  webview overlay working on this machine means writing a new macOS backend (WKWebView via
  `objc`/`cocoa`), not porting this code.

- **`provider.rs`** — the real Escher-`Scaffold`-to-Bevy-UI bridge (maps `Style`/`ElementNode`
  onto `bevy::ui` nodes). Targets the pre-refactor core API (`slate::style::property::BoxSize`,
  `slate::scaffold::Scaffold`, `slate::element::ElementNode`, ~50 symbols total) which no longer
  matches current `escher-core`'s API after the refactor (flat `style.rs`/`element.rs`/etc.
  instead of `style::primitive`/`style::property` submodules, different `Style`/`Property`
  shape). Needs a real rewrite against the current core, not a mechanical rename.

The two example files this crate originally shipped (`examples/basic.rs`, `examples/overlay.rs`,
kept in `examples/` for reference) depend on `provider.rs` plus a `chizel::uix!`/`chizel::styles!`
macro DSL and a component library (`Div`/`Container`/`Header`/`Footer`/`Sidebar`/`TextBlock`/
`Label`/`TextInput`/`Button`) that lived in the old `slate` core. Neither the macro DSL
(`escher-macros`' `uix!` is parsed but not acted on) nor that component library exist in current
`escher-core` — but it turns out neither is actually required: `escher-core`'s `Scaffold` builder
API (`style`/`slot`/`content`) plus its existing `Container`/`Text`/`Input`
elements and `Header`/`Body`/`Footer`/`Legend`/`Content` slot markers are enough to build an
equivalent scene directly, no macro or extra component types needed — see
`../../examples/scene.rs` and `../surface.rs` (`BevySurface`), which render a real, working,
timer-redrawn UI scene against the current core. `provider.rs` itself is still not ported (its
diffing model doesn't match the new core's rebuild-from-scratch `Scaffold` shape anyway — see
`surface.rs`'s doc comment), so it stays here as reference only.

What *did* port cleanly (Bevy 0.15→0.18 API drift only, no core dependency): `plugin.rs`
(trimmed — window/webview/input wiring removed), `config.rs`, `reticle.rs`, `time.rs`, `log.rs`,
and `terminal.rs` (a `ratatui` debug console drawn to the real OS terminal, not the game window —
rough/prototype-quality in the original, kept that way). These live in `src/` and are exercised
by `examples/hello.rs`. `surface.rs` (new, not ported — built fresh against current `escher-core`)
and `examples/scene.rs` are exercised on their own; see above.
