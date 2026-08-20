# Proposal: generalize Scaffold reconciliation across all runtimes

Status: **not implemented — proposal only**, per `AGENTS.md`'s default. Plan requested and reviewed in bullet-list form with the human on 2026-08-15 (response: "Your suggestion for generalizing the reconcile path is good. Go ahead and make a plan"); writing it up as a proper spec doc per the human's follow-up request that future feature talk live in `spec/`, not just chat/agent memory. No code written yet — awaiting explicit go-ahead to implement.

## Current state (verified via search, 2026-08-15)

Only `escher-appkit`'s `AppKitSurface` does real reconciliation today: each node gets a stable `escher_core::scaffold::NodePath` (a `'static`, arena-independent `(TypeId, usize)` key chain), and a node whose path/shape matches the previous draw gets its native object patched in place instead of destroyed and recreated (`spawn_or_patch` in `runtimes/appkit/src/surface.rs`). It still walks the *whole* tree every draw — the saving is in not needlessly recreating native objects, not in skipping work.

`BevySurface` (`runtimes/bevy/src/surface.rs`) and the web DOM surface fully rebuild every draw call (despawn+respawn / recreate DOM nodes from scratch) — confirmed via their own doc comments, which already flag this as a known, accepted limitation, not a bug. `TerminalSurface` also rebuilds its ratatui widget tree from scratch every draw, but see the ROI note below — this one is different.

## Proposed design

**Phase 0 — Define the interface (`escher-core`)**

- New module (e.g. `escher_core::reconcile`) defining a generic reconciliation core, decoupled from any specific native object type or layout system.
- A `Reconciler<K, N>` generic over a surface's own `K` (node-kind classification — AppKit's existing `NodeKind` is the template) and `N` (native object type — AppKit's existing `NativeNode` is the template), taking surface-supplied callbacks:
  - `classify(&Scaffold) -> K`
  - `spawn(&NodePath, &K) -> N`
  - `patch(&mut N, &K)`
  - `same_kind(&N, &K) -> bool`
  - `remove(N)`
- The reconciler owns `HashMap<NodePath, N>` plus the walk/visited-tracking/stale-cleanup logic currently hand-rolled inside `AppKitSurface::draw`/`layout_children` — that bookkeeping is the actual reusable part.
- **Deliberately excludes layout** (frame/position computation) from the shared core. AppKit's points-based flexbox math, Bevy's own `taffy`-based UI layout, and the DOM's own CSS layout engine are different enough that forcing one shape onto all three would fight each of them. The reconciler only walks/classifies/spawns/patches/removes; each surface still runs its own layout pass afterward using whatever native mechanism fits it.
- **Also excludes event dispatch** (the outbox/`NativeEvent` pattern `AppKitSurface` uses) — native-callback shapes differ too much per surface (AppKit target-action vs. Bevy ECS messages vs. DOM event listeners) to usefully share; stays surface-specific.

**Phase 1 — Prove it on AppKit (zero behavior change expected)**

- Refactor `AppKitSurface` to run on top of the new `Reconciler` instead of its own hand-rolled walk, using the same `NodeKind`/`NativeNode` types — just delegating the walk/patch/stale-cleanup to the shared core.
- This is the validation step: if AppKit's existing behavior (including the fixes from this session's other work — text-field-while-typing, active-tab highlight, resize repositioning) survives unchanged, the abstraction is sound. If it doesn't fit cleanly, that means the interface is wrong and needs to change *before* a second surface builds on it — don't proceed to Phase 2 until this refactor is clean.
- Full regression pass against the live tabbed browser (open/close/reorder tabs, typing, resize, collapse) before moving on.

**Phase 2 — Web DOM surface**

- Highest ROI: real DOM node creation/destruction is the most expensive thing currently being redone every single draw.
- Define `NodeKind`/`NativeNode` (`web_sys::Element`) for the web surface, wire it to the shared `Reconciler`.
- Layout stays exactly as-is (the browser's own CSS engine) — only node creation/removal/attribute-patching changes.

**Phase 3 — Bevy**

- Same shape: `NodeKind`/`NativeNode` (`Entity`) for `BevySurface`, wired to the shared `Reconciler`.
- Entity spawn/despawn churn is real but cheaper than DOM churn — hence after Phase 2, not before.

**Phase 4 — Terminal (likely skip or defer indefinitely)**

- Ratatui already buffer-diffs at the *cell* level before writing to the actual terminal — the expensive resource (terminal I/O) is already optimized independent of anything Escher does. A full Scaffold→widget rebuild each draw is cheap CPU work, not a real bottleneck.
- Only worth doing if profiling ever shows otherwise; not scheduled as part of this rollout.

## Cross-cutting

- Update `escher-chalk` compositions only if any of them assume full-rebuild semantics — unlikely, since they're pure `Scaffold`-builder functions with no surface awareness of their own.
- Changelog + memory update after each phase lands, same discipline as this session's AppKit/Anvil work (`spec/.agents/changelog.md`).

## Key risk

The generic reconciler has to stay honestly generic — no AppKit-specific assumptions leaking into the `classify`/`spawn` callback signatures (e.g. AppKit's current `classify()` hardcodes `NodeKind::TabRow` as one of its variants; a properly generic version needs the *whole* classification, not just the generic bookkeeping, to be surface-supplied). If Phase 1's refactor requires contorting the interface to fit AppKit's own quirks, that's a signal to redesign before Phase 2/3, not a signal to push through and special-case it later — the entire point is to avoid each surface reimplementing this from scratch.
