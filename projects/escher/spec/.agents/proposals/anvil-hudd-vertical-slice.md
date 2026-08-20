# Fast vertical slice: Anvil subprocess panes + Hudd toolbar/annotation

Status: revised 2026-08-17 (same day as the first draft). The first version of this doc estimated
three weeks and was checked against actual pace and found to be padded — see "Why this got faster"
below. Explicit target unchanged: **functional MVP, not a polished product** — open-source pace,
cut scope hard rather than gold-plate any one piece. What changed is the schedule, not the scope:
still building the real Anvil pane + Hudd toolbar/drawing + Ethos/Atlas proof described below, just
without three weeks of padding, sequential weeks, and full-fidelity terminal emulation baked in.

## Why this got faster

The three-week estimate was checked against this session's own actual output rather than trusted on
priors: today's uncommitted diff alone (`git diff --stat HEAD -- projects/escher`) is 734 lines of
substantive Rust changes to `apps/anvil/src/main.rs` plus a new `escher-cli` stub, a real
transactional persistence fix, and cleanup across five projects — landed in one sitting, including
a self-audit that caught two real bugs. That's a materially faster per-session rate than a
three-week plan assumes. Three specific places were overpriced:

1. **Full buffer week.** Reserved for "whatever blows up." Real, but sized for a much bigger plan.
2. **Full ANSI/cursor-fidelity for the embedded pane.** A true `vt100` cell-grid parity with real
   cursor addressing is the kind of thing that could eat a week if fought hard. But "workable,
   not fully fleshed out tmux-style embedding" (the actual bar asked for) doesn't need that: raw
   ANSI-passthrough into a scrolling pane, styled via `ansi-to-tui` (already a workspace dependency,
   see `escher/Cargo.toml`), covers most real subprocess output without ever touching
   `portable-pty`/`vt100`/`termwiz` at all. Full cursor-addressable emulation (needed for `vim`,
   `htop`, etc.) is deferred, not required for the vertical slice.
3. **False serialization.** The Anvil pane, Hudd's window/toolbar, and the Atlas channel don't
   depend on each other until final integration — the original plan put them in sequential weeks
   for narrative clarity, not because the work is actually sequential. They can build in parallel
   and only need to meet at the end.

What's still honestly unresolved, not hand-waved away: AppKit click-through event routing for
Hudd's overlay is new to this codebase and untested — it's timeboxed below as a spike with an
explicit fallback, not assumed to just work.

## The actual goal, not just the app

Two things, not one — corrected after an initial pass at this doc scoped only the first:

1. **Ship a working Anvil + Hudd**, per the concrete description below.
2. **Prove Ethos (scripting) and Atlas (connectivity) as real, reusable primitives** other
   developers could build a similar app on — not just internal implementation details hidden
   inside Hudd/Anvil. Every piece of this plan that touches configuration or cross-process
   communication should go through Ethos/Atlas rather than a Rust-only shortcut, specifically so
   the MVP itself is the proof that these two projects are usable building blocks, not just
   plumbing nobody outside Escher could reuse. This is why Atlas is back in scope below — an
   earlier draft of this doc cut it as unnecessary for the app to function, which is true but
   misses the actual point: the app functioning isn't the only deliverable.

## Scope — what's actually being built

1. **Anvil**: real embedded subprocess panes — a child process's live output streamed and rendered
   inside Anvil's own Scaffold/ratatui UI with ANSI styling intact (via `ansi-to-tui`, already a
   workspace dependency), not just raw unstyled text lines. Full cursor-addressable terminal
   emulation (`vt100`/`portable-pty`, needed for `vim`/`htop`-style full-screen programs) is
   explicitly deferred past this slice — "workable, not fully fleshed out" means most subprocess
   output (build logs, REPLs, linear CLI tools) renders correctly; full-screen interactive programs
   don't yet. Plus a real `escher` CLI.
2. **Hudd**: a floating, always-on-top toolbar with a few selectable tools, drawing freehand
   annotations onto a transparent overlay surface. Hudd's own rendering job stops at the toolbar
   and the strokes — it doesn't need to interpret what a "tool" means to whatever's underneath it.
   What changes from the first draft: the toolbar's own tool list/config is authored via an
   **Ethos script** (mirroring the `/shape` pattern already proven in Anvil), not hardcoded in
   Rust — so adding a tool is "write a small script," not "recompile Hudd." And each stroke/tool
   event is published over **Atlas** (the existing `atlas-relay`, already real and tested) as a
   thin, real pub/sub channel — not because Hudd needs a subscriber to function, but because a
   channel nothing can actually subscribe to isn't a proof of anything. Anvil (or a tiny standalone
   example script) is the first real subscriber, closing the loop end-to-end.
3. **A minimal example of someone else's app consuming both** — even something as small as a
   ten-line Ethos script that subscribes to Hudd's Atlas channel and prints what it receives. This
   is the actual "proof," not a claim in a doc — a third party pointed at this example should be
   able to see exactly how they'd plug their own app into the same channel.

**Still out of scope**: Desktop (still a 14-line stub, not mentioned in your scoping either time) —
say so if that's wrong too, but nothing above depends on it existing.

## Current state, verified this session (not assumed)

- **Anvil**: real, working terminal app (Bevy + AppKit + webview), already streams raw subprocess
  stdout into `Page::Process` today — but as plain unstyled text lines, no ANSI handling. `ansi-to-
  tui` is already a workspace dependency (`escher/Cargo.toml`) and unused by Anvil today — wiring
  it into `Page::Process`'s render path is the actual scope of "the pane primitive" for this slice,
  not a new crate to integrate. Zero `portable-pty`/`vt100`/`termwiz` anywhere in the workspace
  (grepped every `Cargo.toml`), and per the scope revision above, this slice doesn't need them —
  full cursor-addressable emulation is deferred, not a blocking unknown for this plan anymore.
- **`escher` CLI**: a one-command stub exists (`apps/cli`, `escher anvil` hardcoded, shells out via
  `cargo run`) — not yet the real cargo/Go-style external-subcommand pattern (`escher-<name>`
  binaries discovered and exec'd, not hardcoded match arms).
- **Hudd**: still a literal 14-line stub crate. But the *hard problem* an earlier session worried
  about for a Hudd-like feature — "always-on-top" — was actually solved for a different, harder
  problem: `always-on-top-window-tool.md` was about forcing an **external** process's window
  (Unity's) to float, which genuinely has no public API and needs private SkyLight symbols. Hudd
  doesn't have that problem — **Hudd owns its own window**, so setting its own level/collection-
  behavior via plain `NSWindow.level`/`collectionBehavior` (public API, already used elsewhere in
  this codebase for Anvil's own scene windows) just works.
- **Ethos scripting, as a proven extension pattern**: `apps/anvil`'s `/shape` command already
  establishes the real shape of "a script authors content, Rust consumes it" (`commands/shape.tsx`
  → JSON description → Rust renders it) — Hudd's tool config is the same pattern applied to a
  different payload, not a new mechanism to invent.
- **Atlas connectivity, what's actually real vs. not**: `atlas-relay` is genuinely working and
  tested (room-based join/leave, message forwarding, a real integration test) — but it's a WebRTC
  *signaling* relay built for peer-to-peer game networking (Aby's use case), not a general pub/sub
  bus. For same-machine Hudd→Anvil communication, the honest options are: (a) actually route
  through `atlas-relay` as a local "room" both processes join — proves the real Aby-proven
  component works for this too, but is arguably the wrong tool (it's solving NAT traversal/peer
  discovery neither process needs on one machine); or (b) use `atlas-store`'s `LocalStore` as a
  shared, polled key-value channel — simpler, but doesn't exercise the relay at all. Decided below
  (the Atlas workstream) rather than left open — `atlas-relay`, with `atlas-store` as a named
  fallback if it fights integration, not a second open question to resolve mid-plan.
- **Real, reusable AppKit precedent**: `escher-appkit`'s `Theme`, hover tracking, and toolbar/tab-
  strip rendering from the Anvil browser work are genuine, working patterns — Hudd's toolbar is
  the same kind of thing, not a new UI paradigm for this codebase.

## The plan: three parallel workstreams, not three sequential weeks

Nothing below depends on the other workstreams until final integration, so they build concurrently
(whether that's multiple sessions in a day, or literally parallel agents on separate pieces).
Sized in a handful of working days total, not weeks — days are effort-sized, not calendar-locked,
since the whole point of the previous section was that per-session output here runs faster than a
week-per-milestone framing assumed.

Named by *what* each one is, not "Track A/B/C" — `docs/src/work-separation-proposal.md` already
uses "Track A"/"Track B" for the two-person split (Lorren/Nasia), and reusing those letters here
for a different axis (Anvil vs. Hudd vs. Atlas) would make the two docs actively contradict each
other the moment someone reads both. Ownership below follows that same doc's split directly: Anvil
and Hudd are both Escher/editor-side work (Nasia's track), the Atlas channel is Atlas (Lorren's).

**Anvil workstream (Nasia) — CLI dispatch + the subprocess pane**
- Real `escher` CLI: rename `apps/anvil`'s binary to `escher-anvil`, change `apps/cli` to look up
  and exec `escher-<subcommand>` instead of a hardcoded match arm. Small, mechanical — hours, not a
  day.
- Wire `ansi-to-tui` into `Page::Process`'s render path so subprocess output keeps its styling
  instead of rendering as plain lines. This is the actual "pane primitive" now that full
  `vt100` cell-grid emulation is deferred — no new crate, no cursor-addressing logic, just using a
  dependency that's already in the tree for its intended purpose.
- Stretch, only if the above lands with room to spare: a basic `portable-pty` spike (child on a
  real pty instead of piped stdout) as a throwaway example, no Scaffold integration — this is the
  one piece of this workstream that's genuinely new territory, so it's explicitly optional rather
  than load-bearing for the slice.

**Hudd workstream (Nasia) — window, toolbar, drawing**
- The window itself first, before any drawing: transparent, correct level/collection-behavior so
  it floats above other apps — closely reusing `escher-appkit` patterns already proven for Anvil's
  own windows.
- Click-through everywhere except the toolbar strip and an actively-drawing region. This is the one
  real unknown in this track — timebox it as a spike; if AppKit's event routing fights past a
  couple hours, fall back to a coarser hit-test region rather than a pixel-perfect one for the
  slice.
- Toolbar: tool list loaded from an Ethos script (2-3 tools — pencil, eraser is enough), themed the
  same way Anvil's browser toolbar already is.
- Drawing: mouse down/drag/up on the transparent surface renders freehand strokes with the selected
  tool — a straight Core Graphics/CALayer path, no pressure sensitivity, no smoothing, no undo
  unless there's time left over.

**Atlas workstream (Lorren) — channel + example consumer**
- Decide the relay-vs-store question now, not "before week 2": use `atlas-relay` — it's already
  real and tested, and doubling up Aby's own networking component for a second, unrelated app is
  the more interesting proof of the two options. Fall back to `atlas-store`'s `LocalStore` only if
  the relay actively fights integration, not preemptively.
- Build the channel and prove it with a dummy event before real strokes exist — publish a fake
  event, confirm a subscriber receives it. Decouples "is the pipe real" from "does drawing work
  yet."
- Finish the example consumer script — a small, real, runnable Ethos script subscribing to Hudd's
  channel and printing what it gets. This is the actual deliverable for "prove it for others," not
  a paragraph in this doc.

**Integration** — once the Anvil workstream has a styled pane, Hudd has real strokes, and Atlas has
a real channel: Hudd publishes real stroke events instead of the dummy one, and the example
consumer prints real data. This is wiring, not new work, if each workstream did its own job.

## Honest risk read, not a confidence claim

CLI dispatch, `ansi-to-tui` wiring, Hudd's window/toolbar, the Ethos-scripted tool config, and the
Atlas channel are all low-risk — closely reusing patterns and libraries already proven in this
codebase. Two real unknowns remain, both isolated to the Hudd workstream: getting click-through
right without fighting AppKit's event routing, and (if the Anvil workstream's stretch goal is
attempted) whether a `pty` child integrates cleanly. Both are timeboxed with an explicit, named
fallback above rather than left open-ended — if either is still fighting past its timebox, take the
fallback and keep moving rather than let it eat the other workstreams' time.
