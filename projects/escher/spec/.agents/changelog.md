# Agent changelog

Per `AGENTS.md`: this logs every change an agent has made outside of `spec/.agents/`, made only with explicit human insistence. Terse — one line per change, git history has the detail. See `spec/.agents/handoff.md` for current in-flight state.

## 2026-08-15 (later)
- Fixed SIGTERM/SIGHUP/SIGINT exit latency: ~13s → ~125ms (`Signals::forever()` + `EventLoopProxy::send_event` wake, was a bare atomic flag waiting on `WinitSettings`' idle-fallback tick). See `proposals/terminal-sigterm-handling.md`.
- Extracted that signal-watching logic out of `apps/anvil` into `escher_bevy::terminal::{spawn_signal_watcher, reraise_signal}` (app code shouldn't own OS-signal plumbing) — re-verified live, ~134ms, no regression.
- Fixed `BrowserState` not resetting when the browser window closes (silent no-op bug on later `/scene` calls).
- Added mdbook + `docker compose watch` docs pipeline to escher's `compose.yaml`/`docs/`; verified live in Anvil.
- Wrote 4 design proposals to `spec/.agents/proposals/` (routing, styleguide, reconciliation, edit-mode).
- Added `spec/.agents/handoff.md` (low-powered-mode convention, see memory).

## 2026-08-12
- Fixed the real animation throttle, retro header, status line moved, two new animations
- Subtle scrollbar, overlay/scrollbar collision, and a real background-fill bug
- Fixed scrollbar thumb position/size math
- Scrollbar widget, mouse-wheel scroll, and a much longer demo seed
- `assistant.rs`: real scrollback, hanging indent, gutter/input alignment
- Style system v1: finished box model, flex grow, typography, scroll
- Gutter alignment + a new `Scaffold::with_overlay` primitive
- `assistant.rs`: real color + Tab-toggled expand/collapse
- `assistant.rs`: bottom-anchored transcript, fixed spacing
- Two more terminal bugs found while interactively verifying the above

## 2026-08-13
- Terminal text selection now trims to real text, and clicking no longer leaves a stray highlighted cell
- Terminal assistant no longer blocks on `sqld`, connects in the background instead
- Dropped the nightly Rust requirement from escher and atlas
- Wired the escher-unity engine into Aby's Unity project
- New `/clear-data` command for the terminal assistant, plus two real bugs found fixing it
- Rewrote comments/docs in the SSG and Docker work against a new house style
- Warnings/dead-code sweep across escher and ethos
- `escher.brainbow.localhost` restored — container back on port 3615, workspace root now the whole monorepo
- `runtimes/web` now runs as a container in escher's docker compose stack
- `runtimes/web`: default scaffold pages to a black page background, not white
- `runtimes/web`: native SSG rendering pipeline, wired into page creation; removed spoofed terminal-assistant reply
- Rebuilt `runtimes/unity` — runs JS in Unity via ethos-ecma, verified end to end from C#
- Built a real `runtimes/os` package: dialogs, app menu, clipboard
- Extracted webview code into its own engine-agnostic crate
- Terminal mouse interaction: widget hit-testing, click-drag selection, clipboard copy
- New `ethos` CLI reuses `escher-bevy`'s `browser` example
- Terminal apps now restore the prior screen on exit, with an optional trace dump
- Terminal + Bevy + native webview running together via a `/scene` command
- Fixed two independent crashes in `ethos-ecma`'s `counter` example
- Built a real, working Bevy UI scene against current `escher-core`
- Ported the Bevy runtime forward from a pre-refactor backup
- Added a way to actually create tasks (`/task <label>`), with a dynamically sized overlay
- Actually fixed the trace-message screen corruption (global redirect, not thread-local); added an FPS display
- Fixed screen corruption from the tokio runtime being multi-threaded
- Real libsql/sqld persistence for `assistant.rs`
- Fixed `swc_allocator`/bumpalo feature conflict from Unity rebuild
- Web runtime: scaffold-backed pages via excalidraw web-embeds
- `@escher/jsx`: a JSX layer for composing scaffolds in Deno
- Terminal assistant: blinking cursor + selection scope/Ctrl+C/focus fixes
- Terminal assistant: slash-command autocomplete + inline highlighting
- Terminal assistant: JS-scripted slash commands (embedded ethos-deno)
- Rejoined escher-unity to escher's main workspace
- Fixed the real Padding+Border+BackgroundColor bug, args highlighting, terminal-restore hardening
- Fixed stale characters in the overlay's padding gap (Clear widget)
- Dropped the fake demo seed, real data only; task selection swaps the Body area

## 2026-08-14
- Terminal assistant: F2-toggled raw tracing firehose page
- Terminal assistant: single-row header, blank gap above the input box
- Resize handle now has a visible grip glyph
- Terminal floating overlay is now draggable and resizable with the mouse
- TerminalApp restores the terminal on SIGTERM/SIGHUP/SIGINT, not just in-process panics
- Terminal assistant: live tracing stream into the transcript, JS command progress reporting
- Terminal assistant: cursor no longer sticks on trailing whitespace; autocomplete moved into its own bar above the input
- `escher-webview` follows the OS's light/dark appearance
- `/clear-data` renamed to `/clear`, misleading `<args>` hint removed
- Terminal assistant's JS slash commands no longer freeze the UI
- escher-unity's C# codegen follows ethos's dialect/runtime split

## 2026-08-15
- Full Scaffold-based AppKit renderer; tabbed browser for `/scene`; `escher-chalk`
- SwiftUI chrome bar mounted into the `/scene` browser scene
- `escher-os`: quick-and-dirty SwiftUI chrome bar stub
- Terminal assistant: raw subprocess stdio, viewable on its own page (F3)
- Terminal assistant: overlay position now persists via sqld
- `terminal_scene.rs`'s Bevy window no longer opens before a `/scene` command
- Found and fixed two real performance/behavior bugs in the live `assistant` app
- Raw trace stream mode (F1 / `--no-tui`), and two bugs it surfaced
- Hoisted the terminal+Bevy+webview app out of `examples/` into `apps/anvil`
- Real render/event-loop performance fix: found and removed the actual bottleneck
- Fixed: closing the scene window used to exit the whole app
- Redesigned scene windows: each `/scene` opens a real independent window, not one shared window
- Dark mode: removed the CSS invert-filter hack, kept the real fix; chrome-bar visibility still broken
- Chrome-bar visibility bug: does not reproduce on a fresh build
- Two real fixes: input render lag, and Google/YouTube serving reduced UI

## 2026-08-15 (styleguide v1 + AppKit chrome theming)
- Added `packages/styleguide` (`escher-styleguide`) — minimal markdown+YAML-frontmatter parser,
  colors/dimensions only, no aliasing/W3C tagging yet. See `proposals/styleguide-frontmatter.md`'s
  "What actually shipped" section for the full scope cut.
- `apps/anvil/anvil.styleguide.md` — real token file (Tokyo-Night-derived palette); terminal's
  previously-hardcoded color consts now read from it (fallbacks kept).
- `escher-appkit` gained a minimal `Theme{background,surface,accent,text}` on `AppKitSurface` —
  themes the toolbar/tab-strip root background, active-tab highlight, text/textfield colors.
  Verified live: dark chrome, blue accent tab highlight, matches TUI palette.
- Follow-up from live feedback ("clunky and ugly/inconsistent"): toolbar buttons switched from
  default `NSBezelStyle::Push` chrome to flat/borderless + tinted (themed surfaces only); address
  field now gets a themed background instead of default white. Verified live via screenshot.
- Fixed the "clunky, UI doesn't update for a while after clicking" perf complaint at its actual
  root: native AppKit toolbar/tab-strip callbacks (button clicks, tab selects, text submits) never
  woke Bevy's `WinitSettings::desktop_app()`-throttled event loop — same root cause class as the
  SIGTERM latency bug, same fix shape (`AppKitSurface::set_wake_callback`, wired from
  `escher_appkit::bevy` via `EventLoopProxyWrapper`). Not live-verified via automated click (a
  coordinate-based test click landed on an unrelated iTerm2 window instead of Anvil — stopped
  rather than retry blind coordinates; no keystrokes sent, nothing else observed disrupted). Worth
  the user trying a real click themselves to confirm.
- Hover + cursor pass (scoped fast path, no real token-coverage/animation work — see "what it
  would take" answer logged in this session, not written to a proposal doc): new
  `runtimes/appkit/src/hover.rs` (`HoverTarget`, an `NSTrackingArea`-owning `NSObject` bridging
  `mouseEntered:`/`mouseExited:` into a Rust closure, mirrors `ActionTarget`'s reason for existing).
  Wired into themed toolbar buttons (pointing-hand cursor + text→accent tint swap on hover) and
  directly into `TabRowView` (own tracking area, since it's already a custom subclass — dims the
  selected-tab highlight color to 35% alpha on hover for non-selected rows). Deliberately instant,
  not eased/animated — real transitions need a timer-driven redraw loop that doesn't exist yet, and
  weren't judged worth it for "decent-looking, quickly." Verified: compiles clean, live screenshot
  confirms toolbar/tab-strip render correctly with the new code paths in place; hover *interaction*
  itself not live-verified (would need synthetic mouse-move, same coordinate risk as the click test
  above — skipped for the same reason).
- Sizing/breathing-room pass, from live feedback (icons small, hit areas tight): toolbar buttons
  28→34pt wide, gap 8→10, side padding 10→14; tab rows 32→38pt tall, favicon 16→18, close button
  18→22, gap 6→8, side padding 8→10; themed-button font bumped to 15pt (was default system ~13pt).
  `TOOLBAR_HEIGHT` (44) was already generous — untouched.
- Real loading-progress feedback, closing the "clunky, not proactively updating" gap: new
  `WKNavigationDelegate` conformance in `runtimes/webview/src/macos.rs` (`NavigationDelegate`,
  polls-not-pushes an `Arc<AtomicBool>` — matches this crate's existing `can_go_back`-style poll
  API instead of adding new Bevy event plumbing). `WebView::is_loading()` new public method.
  Threaded through: `ToolbarState.loading: bool` (escher-appkit::bevy) into `escher_chalk::toolbar`'s
  new `loading` param, which swaps the refresh glyph — visual-only, still just calls `on_refresh`,
  not a real Stop action. Anvil's `sync_toolbar_state` sets it from the active tab's own
  `WebView::is_loading()` every tick. Verified: workspace `cargo check --all-targets` clean; visual
  screenshot verification of this specific increment was inconclusive — `set frontmost` targeted a
  second, pre-existing anvil process (PID 20893, showing YouTube) instead of the freshly-built test
  instance, since macOS picks an arbitrary match when two processes share a name. Left that process
  untouched (unclear if it's an earlier test of mine that didn't fully clean up, or something else)
  rather than guess-killing it.
- Text/sizing pass, from live feedback ("doesn't render text very well"): `escher-styleguide`
  gained a `text: HashMap<String, f64>` token category (font sizes in points) alongside `colors`/
  `dimensions` — `anvil.styleguide.md` now defines `text.ui: 15` (button glyphs) / `text.body: 13`
  (labels/address field). `escher-appkit`'s `Theme` gained `ui_text_size`/`body_text_size`, applied
  to every text-bearing control (previously only buttons had an explicit font — labels and the
  address field silently used whatever the OS default happened to be, which is a real source of
  "doesn't render well": nothing guaranteed they matched). Also fixed: tab-title `Label`s had no
  line-break/truncation mode set, so a title that didn't fit its slot wrapped to a second line and
  got clipped inside the fixed-height row instead of truncating with an ellipsis — now
  `NSLineBreakByTruncatingTail` + single-line mode. Also fixed: the loading-indicator glyph added
  earlier (`\u{25CC}` DOTTED CIRCLE) was a Unicode combining-mark placeholder, not a standalone
  character — renders as a tiny malformed dot in most fonts; replaced with `\u{25D0}` (a real
  Geometric Shapes character). Verified: workspace `cargo check --all-targets` clean, `escher-
  styleguide`'s tests pass, live screenshot (this time with the correct window precisely targeted
  by PID via `System Events... whose unix id is <pid>`, avoiding the earlier
  `first process whose name is "anvil"` ambiguity) confirms clean single-line tab-title text at the
  new consistent size. Terminal side: confirmed there's no equivalent "font size" concept to
  tokenize — the terminal's actual font/size is the user's own terminal emulator's, outside this
  app's control; only its *colors* are styleguide-driven, which was already true before this pass.
  Note left in `anvil.styleguide.md` explaining this split.
- Added `spec/ROADMAP.md` — living milestone checklist (M0-M10 + adjacent items) covering
  everything discussed/scoped this session (reconciliation generalization, styleguide extension,
  Ethos dialects, edit-mode, Hudd/Desktop-as-stubs, CI/publishing, docs). Meant to be updated going
  forward as the primary cross-session progress tracker, human- and agent-facing both — supersedes
  ad hoc status recaps in this changelog for "where are we overall" questions.
- Added `packages/edit` (`escher-edit`) — the `EditBackend` trait + `InMemoryEditBackend` stub,
  designed as the seam between an editor-UI build-out and the eventual Ethos-backed codegen
  implementation (see `docs/src/work-separation-proposal.md`). `insert`/`delete`/`move_node`/
  `set_style`/`set_content`/`commit`, keyed by `NodePath`. Deliberately minimal — no undo/redo,
  no transactions. Tested (`insert_then_delete_round_trips`), workspace `cargo check
  --all-targets` clean.
- Added `docs/src/work-separation-proposal.md` (mdbook, live at `localhost:8096` — verified 200)
  — presentable write-up of the Lorren/Nasia track split: Track A (Ethos dialects, Atlas, Escher
  core) vs. Track B (apps/examples catch-up, then editor UI up to "not yet backed by real Ethos
  codegen/advanced Atlas tooling"), the `escher-edit` seam that makes the split safe instead of
  just relabeling "wait on Ethos," and named coordination risks (core API churn, contract drift).
- Anvil: tab-owned loading state. `Tab` gained its own `loading: bool` field, refreshed each tick
  by a new `sync_tab_loading_state` system from that tab's own `WebView::is_loading()`.
  `sync_toolbar_state` now just reads `browser.active_tab().loading` instead of reaching into
  `TabWebViews` directly — nav-relevant state lives on the tab itself, not recomputed ad hoc
  wherever it's needed (also sets up per-tab loading indicators in the tab strip later, not just
  the active tab's toolbar). From live feedback requesting the toolbar's data "live inside the
  tab" for better data flow — flagged honestly to the user that this is unlikely to be the actual
  cause of new-tab-creation lag (more likely AppKit tab-strip reconciliation / webview-attach
  cost), but is a real, independently-justified data-ownership improvement regardless. Workspace
  `cargo check --all-targets` clean; not live-verified this pass (end of session).
- Compressed this changelog to terse one-line-per-entry format (2,760 → 186 lines; 61 old
  pre-convention prose sections grouped by day). Documented the convention in `AGENTS.md` itself
  so it's not just implicit from this file's own header, and checked both off in `spec/ROADMAP.md`.
