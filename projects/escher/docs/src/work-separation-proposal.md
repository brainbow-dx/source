# Work Separation Proposal — Escher, Two Tracks

Drafted 2026-08-15. Proposed split of Escher work across two people, each running their own
Claude Code agents at an agreed cadence, so both stay busy without stepping on each other or
blocking on work the other hasn't finished yet.

## The split

**Track A (Lorren):** Ethos dialects, Atlas, and Escher core.

**Track B (Nasia):** Bring the existing apps/examples up to speed, then build out the Escher
editor's UI — everything visual and interactive, up to the exact line below.

## The editor boundary, precisely

Track B's editor work covers **all visual UI for the full editor experience** — layout, panels,
selection, insert/delete/move interactions, the works — **not yet backed by the real Ethos
codegen backend or advanced Atlas tooling**, beyond whatever the two tracks build out together
along the way as a fast follow.

In practice: Track B builds and demos the editor's UI against an in-memory stand-in that mutates a
runtime tree only — nothing reaches disk. Track A's job is to make the real thing (Ethos-backed,
writes precise source-code patches) satisfy the exact same contract, so swapping one for the other
is a one-line change at whichever call site constructs the backend, not a rewrite of UI code.

That contract is the thing that makes this split safe rather than a rename of "wait on Ethos."
Without it, Track B either sits idle until Track A's codegen pipeline lands, or invents its own
shape that gets thrown away once the real one exists. With it, both tracks can move now.

## The seam: `escher-edit`

New crate, `packages/edit` (`escher-edit`) — one trait, two implementations expected over time.

```rust
pub trait EditBackend {
    type Error: std::error::Error;

    fn insert(&mut self, parent: NodePath, index: usize, node: NodeEdit) -> Result<NodePath, Self::Error>;
    fn delete(&mut self, path: NodePath) -> Result<(), Self::Error>;
    fn move_node(&mut self, path: NodePath, new_parent: NodePath, index: usize) -> Result<(), Self::Error>;
    fn set_style(&mut self, path: NodePath, property: Property) -> Result<(), Self::Error>;
    fn set_content(&mut self, path: NodePath, content: Option<String>) -> Result<(), Self::Error>;

    /// Flushes any in-flight/uncommitted edits.
    fn commit(&mut self) -> Result<(), Self::Error>;
}
```

- **`InMemoryEditBackend`** — implemented today, ships with the crate. Pure in-memory, no
  persistence. This is what Track B's editor UI builds and demos against, starting now.
- **Ethos-backed implementation** — Track A's to build. Buffers edits and turns `commit()` into
  one precise source-code patch via Ethos's codegen, once that pipeline exists. Same trait, same
  call sites — the editor UI shouldn't need to change at all when this lands.

Selection/hit-testing (turning a click into a `NodePath`) stays a runtime-UI concern, outside this
trait — `NodePath` (stable `(TypeId, usize)` identity, already used throughout Escher's
reconciling surfaces) is what a UI resolves a click down to before calling any of the above.

Deliberately minimal for now: no undo/redo, no batching/transactions, no concurrent-edit conflict
resolution. Add those once a real consumer actually needs them.

## Coordination risks worth naming up front

- **Core API churn.** Track A is actively reshaping Escher core while Track B builds apps and
  editor UI on top of it. Agree on a sync point (daily standup, or just a heads-up in chat before
  landing a breaking core change) so Track B doesn't discover drift after the fact.
- **The `EditBackend` contract itself.** It's sketched above and already compiles
  (`packages/edit`), but it's a first draft — expect it to need small adjustments once Track B
  actually builds insert/delete/move UI against it. Track A should expect (and welcome) that
  feedback rather than treating the trait as frozen.
- **"Beyond what we build out together"** is intentionally vague — the boundary is meant to move
  as both tracks make progress, not stay fixed at today's line. Revisit it regularly rather than
  treating this document as the final word.

## What this split does and doesn't compress

Running two tracks buys a better milestone sooner: solid Ethos/Atlas/core infrastructure *and* a
caught-up app suite *and* real editor-UI progress, all around the same time. It does **not**
compress the critical path to the full vision — the Ethos Rust-targeting codegen dialect is still
a sequential dependency for anything beyond the in-memory stand-in, regardless of headcount. The
point of this split is parallel progress on everything that doesn't need to wait for it, not a
faster finish line on the parts that do.

See `spec/ROADMAP.md` in the repo for the fuller milestone breakdown this plugs into (M4 — Ethos
Integration, M5 — Scaffold Edit Mode specifically).
