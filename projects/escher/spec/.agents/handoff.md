# Handoff — 2026-08-15 (end of day)

Overwritten each time there's a natural stopping point (quota-driven or, like today, the human
signing off for the day). Current entry only; don't append history here — see `changelog.md` for
the full trail, or `spec/ROADMAP.md` for the standing cross-session milestone tracker (start
there first, it's more current than this file will be by next session).

## Primary tracker going forward

**`spec/ROADMAP.md`** — living milestone checklist (M0-M10 + adjacent items), meant to be updated
as work lands rather than re-derived each session. Check it before assuming anything below is
still accurate.

## Done today, verified working

- Styleguide v1 (`escher-styleguide`): colors/dimensions/text tokens, applied to Anvil's terminal
  colors and AppKit toolbar/tab-strip chrome (background, accent, text, sizes) — same source of
  truth for both surfaces.
- AppKit chrome iteration, all verified live via screenshot: flat borderless toolbar buttons
  (replacing default `NSBezelStyle::Push`), themed dark address field/tab-strip background, hover
  states (pointing-hand cursor + tint/highlight via new `runtimes/appkit/src/hover.rs`), sizing
  pass (bigger buttons/rows, more padding), text-rendering fixes (consistent font sizes, tab-title
  truncation, a bad Unicode glyph swapped out).
- Click/tab-switch responsiveness: native AppKit callbacks now explicitly wake Bevy's throttled
  event loop (`AppKitSurface::set_wake_callback`) — same root-cause family as the earlier SIGTERM
  fix, just never applied to this path before.
- Real webview loading-progress feedback: new `WKNavigationDelegate` in `escher-webview`
  (`WebView::is_loading()`), threaded to the toolbar's refresh-button glyph.
- Tab-owned loading state: `Tab.loading` refreshed each tick by `sync_tab_loading_state`, toolbar
  reads it instead of reaching into `TabWebViews` directly — from live feedback about wanting
  nav-relevant state to "live inside" each tab; flagged to the user this probably isn't the fix
  for new-tab-creation lag specifically (more likely AppKit tab-strip reconciliation/webview-
  attach cost), just an independently-good data-ownership change.
- `packages/edit` (`escher-edit`, new crate): `EditBackend` trait + `InMemoryEditBackend` — the
  seam between an editor-UI build-out and the eventual Ethos-backed codegen implementation. Built
  in response to a two-person work-split discussion (see below).
- `docs/src/work-separation-proposal.md` — presentable mdbook page (verified live at
  `localhost:8096`) for the human to bring to a work-split conversation tomorrow.
- `spec/ROADMAP.md` — new living milestone doc, M0 through M10 plus adjacent items, meant to
  replace ad hoc "where are we" recaps going forward.
- Diagnosed (not a code fix — a usage/docs issue): the `escher-web` drawing-canvas dev server
  (Docker container `escher-web-1`, port 3615) mounts the *entire* `Brainbow` directory as its
  workspace root, not just the `escher` repo — every URL path needs a `projects/escher/` prefix.
  Confirmed by reproducing the 404 both ways. Worth adding this to the drawing-canvas's own docs
  so it doesn't surprise someone else the same way.

## Open, unresolved

- **Styleguide "component dependencies" schema question** — still unanswered (blocks the
  W3C-token-format / aliasing work, not blocking anything else). See
  `proposals/styleguide-frontmatter.md`'s bottom section.
- **Reported overlay-window-drag hang** — still not reproduced or diagnosed.
- **Second stray Anvil process** seen repeatedly during today's live-verification screenshots
  (PID 20893, showing YouTube) — never touched (per the never-blind-kill rule), never identified.
  Worth the human checking what it is.
- **CI/publishing scope** — still waiting on the human's answer (which registries per project,
  GitHub Actions confirmed?, what Pages should serve).

## Not started (see `spec/ROADMAP.md` for the fuller list)

- Real eased/animated hover transitions (deliberately deferred — needs a timer-driven redraw loop
  that doesn't exist).
- Full `Property`-surface styleguide coverage (only colors/dimensions/text today).
- Generalized reconciliation for Bevy/Web surfaces (AppKit's is the only one that reconciles
  in-place; proposal written, not started).
- Changelog compression to terse format (older long-form entries still pending — new entries
  already terse, going forward).
- Ethos Rust dialect (the M4 long-pole item — nothing scheduled yet this session).

## Immediate next step if resumed

No single blocking thread — multiple independent options, roughly equal size:
1. Answer the styleguide component-dependencies question (unblocks that whole sub-feature).
2. Start on the Ethos Rust dialect (the actual critical-path item per the roadmap's own
   sequencing notes).
3. Continue the work-split conversation once the human has talked to Nasia — may reshape
   priorities entirely depending on what she picks up.
Ask which, don't assume — this session ended on the human's own explicit sign-off ("about to go
watch a movie"), not a quota/forced stop, so there's no single obviously-correct next move baked
in from context the way there usually is.
