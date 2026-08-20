# Escher Roadmap

Living checklist toward "fully functional, polished." Update as work lands — check items off,
add new ones, don't let it drift stale. Rough, not exhaustive; see linked proposals in
`spec/.agents/proposals/` for design detail behind any item. Status tags: ✅ done & verified,
🚧 in progress / partial, ⬜ not started, 🔒 blocked on something listed after it.

Last updated: 2026-08-15.

## M0 — Scaffold Core (foundation everything else sits on)

- [x] ✅ `Scaffold`/`StyleSheet`/`Property` core (`packages/core`) — margin, padding, size, gap,
      flex/flex-direction, heading, font-style/weight, text-decoration/align, overflow,
      content/background color, border, scroll-position, overlay-inset
- [x] ✅ Stable `NodePath` identity across draws (the `with_slot` per-type-counter fix)
- [x] ✅ AppKit reconciling renderer (`runtimes/appkit`) — patches native views in place, doesn't
      rebuild the tree every draw
- [ ] ⬜ Bevy surface reconciliation (currently full teardown/rebuild every draw — see
      `proposals/generalized-reconciliation.md` Phase 3)
- [ ] ⬜ Web/DOM surface reconciliation (Phase 2 of the same proposal)
- [ ] ⬜ Generic `Reconciler<K, N>` extracted to `escher-core` so AppKit/Web/Bevy share one
      implementation instead of three (`proposals/generalized-reconciliation.md` Phase 0-1)
- [ ] ⬜ Terminal reconciliation — likely skip per the proposal's own Phase 4 note (ratatui's own
      diffing may already be enough)

## M1 — Anvil Browser (toolbar / tabs / webview)

- [x] ✅ Tabbed browser shell: toolbar (back/forward/refresh/address), vertical tab strip,
      per-tab `WebView` (state persists across tab switches), collapsible sidebar
- [x] ✅ Global back/forward/refresh shortcuts (mouse side-buttons, Cmd+[/Cmd+]/Cmd+R)
- [x] ✅ Resize-stable toolbar/tab-strip positioning (`Pin`/`reposition`, no more autoresizing-mask drift)
- [x] ✅ Click/tab-switch responsiveness fix (native callbacks now wake the throttled event loop)
- [x] ✅ Real UA spoofing option so Google/YouTube serve modern UI, not legacy fallback
- [x] ✅ Loading-progress feedback (`WKNavigationDelegate` → `WebView::is_loading()` → toolbar glyph)
- [x] ✅ Sizing/hit-area pass (bigger buttons/rows, more breathing room)
- [x] ✅ Text rendering pass (consistent font sizes everywhere, tab-title truncation, bad glyph fix)
- [x] ✅ Hover states: pointing-hand cursor + instant tint/highlight on buttons and tab rows
- [x] ✅ Tab-owned loading state (`Tab.loading`, not recomputed ad hoc from `TabWebViews`) —
      better data ownership; not expected to be the actual fix for new-tab-creation lag
- [ ] ⬜ Real eased/animated hover & state transitions (needs a timer-driven redraw loop — none
      exists yet; deliberately deferred, instant-swap only so far)
- [ ] ⬜ Dark-mode verification signed-in (Google/YouTube dark theming looked gated behind account
      state, not `prefers-color-scheme` — unconfirmed, needs a real signed-in check)
- [ ] ⬜ Investigate reported hang when moving/dragging the overlay window (not reproduced yet)
- [ ] 🔒 Browser extensions (needs a JS engine wired to `escher-webview` — see M4)

## M2 — Styleguide System

- [x] ✅ v1: markdown+YAML-frontmatter parser (`escher-styleguide`) — flat `colors`/`dimensions`/`text` tokens
- [x] ✅ Applied across Anvil: terminal color consts + AppKit toolbar/tab-strip chrome read the
      same `anvil.styleguide.md`
- [ ] ⬜ Full `Property` coverage — border/typography/layout tokens (margin, padding, flex, etc.),
      not just colors/dimensions/text (see "what it would take" note in `proposals/styleguide-frontmatter.md`)
- [ ] ⬜ Variable/token aliasing (`{token.path}` references, resolved at load time)
- [ ] ⬜ W3C Design Tokens-shaped `$type`/`$value` tagging (long-term CSS-parity target)
- [ ] ⬜ **Open decision needed from human:** "component dependencies" semantics — styleguide
      `extends`/imports vs. component-declares-required-tokens, or both (blocks the above two)
- [ ] ⬜ Compile-time codegen (Rust constants from a styleguide, instead of stringly-typed lookups)
- [ ] ⬜ Apply to Bevy scene + webview-content surfaces (currently only terminal + AppKit chrome —
      webview *content* isn't ours to restyle; Bevy scene untouched so far)

## M3 — App-Level Concerns

- [ ] ⬜ App-level view routing (`ViewStack<V>` + `Navigate<V>` pattern — proposal written, not built)
- [ ] ⬜ Scaffold→ANSI headless rendering (for dev scripts/docs, no live window needed) — cheapest
      unstarted item, good next pick
- [ ] ⬜ Terminal drawing performance baseline (mouse-drag-flood throughput, untested)
- [ ] ⬜ Excalidraw-style example running inside `escher-terminal`

## M4 — Ethos Integration (unlocks edit-mode + extensions + scripting)

- [ ] ⬜ Rust-targeting Ethos dialect, scoped to Scaffold call-site edits (insert/delete/rename on
      `.with_style()`/`.with_slot()`, not general refactoring) — the long pole for M5
- [ ] ⬜ Shell/bash dialect (spec written: `projects/ethos/spec/agents/proposals/shell-dialect.md`)
      — minimal subset ~2-4 days, real bash compat ~2-4 weeks (human-effort estimate)
- [ ] ⬜ Decide `ethos-deno` consumption path for browser extensions (existing FFI/C-ABI boundary
      vs. direct Rust link) — see `proposals/anvil-browser-extensions-ecma.md`
- [ ] ⬜ Real stdio/PTY embedding in bounded Scaffold regions (`portable-pty` + `vt100`/`termwiz` —
      confirmed not to exist yet, needed for Anvil `--tui` dockable overlay)
- [x] ✅ `EditBackend` seam (`packages/edit`, `escher-edit`) — trait + `InMemoryEditBackend` stub,
      the contract an editor-UI track builds against now and the real Ethos-backed implementation
      satisfies later. See `docs/src/work-separation-proposal.md`.

## M5 — Scaffold Edit Mode

🔒 The real, persisted version is still blocked on M4's Rust dialect. **UI-layer work is now
unblocked** via `EditBackend`/`InMemoryEditBackend` (M4) — an editor-UI track can build and demo
insert/select/delete/restyle interactions today against the in-memory stub, swapping in the real
Ethos-backed implementation later without changing UI call sites.

- [ ] ⬜ Design doc (`proposals/scaffold-edit-mode.md` — written, no code)
- [ ] ⬜ Selection/hit-testing in one runtime (AppKit — furthest along, has `NodePath` already)
- [ ] ⬜ Insert/delete/move interactions wired to `EditBackend` (stub first, Ethos-backed later)
- [ ] ⬜ Ethos-backed `EditBackend` implementation (turns `commit()` into a precise source patch)
- [ ] ⬜ Runtime-side in-flight-edit database (for edits not yet flushed to disk)
- [ ] ⬜ Roll out to Hudd, Desktop once each is a real app (see M7/M8)

## M6 — CLI & Process Plumbing

- [x] ✅ SIGTERM/SIGHUP/SIGINT graceful exit (~125ms, extracted to `escher_bevy::terminal`)
- [ ] ⬜ Real `clap`-subcommand-based `escher` CLI (`escher anvil`, cargo-style) — not a flag
      bolted onto one binary; explicitly required before the Anvil `--tui` item above ships
- [ ] ⬜ Anvil `--tui` + dockable subprocess overlay (tmux-like), depends on the CLI item + PTY
      embedding (M4)

## M7 — Hudd (always-on-top overlay daemon)

- [ ] ⬜ Still a 14-line stub crate — not a real app yet
- [ ] ⬜ Global-hotkey summon (current `GlobalShortcuts` is local-monitor-only, not a true
      system-wide hotkey — confirmed gap)
- [ ] ⬜ Per-monitor/per-desktop positioning
- [ ] ⬜ Edit-mode support (once M5 exists)

## M8 — Desktop (persistent per-virtual-desktop surface)

- [ ] ⬜ Still a 14-line stub crate — not a real app yet
- [ ] ⬜ Core persistent-surface behavior
- [ ] ⬜ Edit-mode support (once M5 exists)

## M9 — Docs & Tooling

- [x] ✅ mdBook + `docker compose watch` live-reload docs pipeline, rolled out to all 6 top-level
      Brainbow projects (aby/atlas/escher/ethos/stooper/eden)
- [x] ✅ Escher's own docs verified live inside Anvil's browser
- [ ] ⬜ Brainbow-level `templates/docs/` extraction — reportedly done by a parallel session,
      **not independently verified by this one**
- [x] ✅ Changelog compression to terse one-line-per-entry format (2,760 → 186 lines; the 61
      old pre-convention prose sections grouped by day as one-liners)
- [x] ✅ `AGENTS.md` updated to state the terse-changelog convention explicitly

## M10 — Repo-Wide CI / Publishing

🔒 Blocked on scope decision from human — not yet answered.

- [ ] ⬜ Which packages publish where (crates.io / npm / JSR / container registries), per project
- [ ] ⬜ CI target confirmed (GitHub Actions assumed, not confirmed)
- [ ] ⬜ What GitHub Pages should serve (the mdbook docs already built, or something else)
- [ ] ⬜ PR workflow / branch strategy for the monorepo

## Adjacent, not blocking Escher itself

- [x] ✅ Aby migrated into the monorepo (`projects/aby/runtimes/unity`) — unverified in a real
      Unity Editor session yet
- [x] ✅ Smash & Stab cleaned up as its own snapshot (`sandbox/experiments/smashandstab`),
      separated from Aby's failed merge, compiles clean — manual playtest still pending
- [ ] ⬜ Universal drawing-tools vision (spans Desktop/Hudd/Anvil/editor tools —
      `proposals/universal-drawing-tools.md`, vision only, nothing built)

---

## Rough sequencing (what unblocks what)

1. **M0's reconciliation generalization** and **M2's styleguide extension** can happen anytime,
   independently — no blockers, good filler work.
2. **M4's Ethos Rust dialect** is the single biggest unlock: it gates M5 (edit mode) entirely, and
   indirectly matters for a "polished, editable" app. Start this early given how long it'll take.
3. **M6's real CLI** gates the Anvil `--tui` overlay item and is small — worth doing soon so later
   work builds on the right entrypoint shape instead of another retrofit.
4. **M7/M8 (Hudd/Desktop)** are both still stub crates — becoming real apps is a prerequisite to
   almost everything else scoped for them, including edit-mode rollout.
5. **M10 (CI/publishing)** needs a human decision before any of it is actionable — flag again next
   session if still unanswered.
