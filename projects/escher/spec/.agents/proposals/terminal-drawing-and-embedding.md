# Proposal (consolidated, terse per low-powered-mode): terminal drawing, stdio embedding, Hudd, script-embedded TUI blocks

Status: **not implemented — proposal only**, per `AGENTS.md`'s default. Five related asks from one 2026-08-15 conversation, written up together and tersely per the human's own low-powered-mode instruction (quota was tight when these arrived) — each deserves its own fuller proposal once picked back up, this is a placeholder capturing the ask + assessment so nothing is lost.

## 1. Isolated terminal-perf baseline (Brainbow `experiments/` folder)
Bare-minimum terminal app, no Bevy/webview/other system-tree work competing for the thread — proves whether the core `escher-terminal` pipeline can sustain drawing-quality frame rates in isolation. This session's fix (`TerminalSurface::draw_with_poll_timeout(Duration::ZERO)`) already proved keystroke-burst throughput; **mouse-drag event floods are untested** and are a different load shape. Build the isolated example, drive it with a synthetic or real mouse-drag flood, measure — don't assume it transfers from the keystroke fix.

## 2. Smallest-viable Excalidraw clone as an `escher-terminal` example
Downstream of #1. Terminal cells are coarse for freehand drawing — real fidelity needs sub-cell rendering tricks (Unicode braille/half-block, as `drawille` and similar do). Scope small first: straight lines/shapes before freehand curves.

## 3. Stdio/subprocess embedding in bounded Scaffold regions
**Does not exist today** — confirmed, not assumed. Needs: a real PTY per embedded child (`portable-pty` is the standard crate), an ANSI/terminal-state parser translating the child's output stream into a cell grid (`vt100` or `termwiz`), and a Scaffold-side element/marker that a surface (`escher-terminal` first, `escher-appkit` later for an embedded-terminal overlay) knows how to render that grid into a bounded rect. This is the real, load-bearing primitive #2's "hoist to Anvil" and #5 both depend on — worth scoping as its own full proposal before building.

## 4. Anvil: `escher anvil --tui` + dockable subprocess overlay (tmux-like)
Depends entirely on #3 existing first. Separately, check whether Anvil's current CLI arg handling actually supports starting terminal-only (skipping Bevy/webview entirely) — this session's work always assumed both start together; that split needs verifying, not assuming.

**CLI shape, per the human's explicit correction**: not a flag tacked onto one binary — a proper top-level `escher` CLI with subcommands (`escher anvil`, presumably others alongside it), wired the way `cargo` wires its own subcommands, via `clap`'s subcommand support (`#[derive(Subcommand)]` or equivalent). `--tui` becomes a flag *on* the `anvil` subcommand, not a standalone invocation shape. This likely means a new top-level `escher` binary/crate that dispatches to `apps/anvil` and whatever else eventually joins it, rather than `apps/anvil`'s own binary growing ad hoc flags indefinitely — worth confirming that structure before writing any of it.

## 5. Hudd daemon viability reassessment
Meaningfully closer than before this session (`AppKitSurface` rendering + reconciliation + native window control are all proven working). **Gap, confirmed not covered**: `escher_appkit::shortcuts::GlobalShortcuts` uses a *local* `NSEvent` monitor (`addLocalMonitorForEventsMatchingMask:handler:`) — only fires while the app already has focus. iTerm's Cmd+`-style "summon from any app" needs a real global monitor (`addGlobalMonitorForEventsMatchingMask:handler:`) or a `CGEventTap`, plus Accessibility/Input Monitoring permission entitlements on modern macOS — genuinely unbuilt, and the actual hard part of this feature. Also needs `NSWindow` floating-level + all-spaces collection behavior, untested (this session's `AlwaysOnTop` usage was narrower — debug-build-only, single-space).

## 6. Scaffold-rendered TUI blocks printable from plain dev scripts (dax/shell-script context)
Smallest lift of the five, architecturally. Ratatui already supports headless rendering to a `Buffer` (its own `TestBackend` does exactly this for tests) — repurposing that to render a `Scaffold` tree to a plain ANSI string, then a normal `print()`, no raw mode/alternate screen needed, reuses nearly all existing Scaffold→ratatui composition logic. `escher-jsx` (currently an empty package, confirmed via earlier dependency audit) is presumably the intended authoring syntax for this specific use case — worth confirming with the human before assuming that's its purpose.

## Suggested order if picked up together
6 (cheapest, no new primitives) → 1 (cheap, informs everything else) → 3 (the real unlock, unblocks 2/4) → 2/4 (build on 3) → 5 (independent, but the global-hotkey gap is its own real chunk of work).
