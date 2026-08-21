# Proposal: Scaffold "edit mode" across all runtimes, backed by Ethos

Status: **not implemented — proposal only, and explicitly blocked on external work**, per `AGENTS.md`'s default. Written up from a design discussion with the human on 2026-08-15; no code was written for this. Unlike the other three proposals from this same discussion, this one is **not ready to schedule** — it depends on Ethos infrastructure that doesn't exist yet (see "Blocker" below).

## The ask

Anywhere an Escher surface lives, the app runtime should support an "edit" mode for the `Scaffold`s running through it: a thin, per-runtime layer allowing user interactions for scaffold editing — insertion, selection, deletion, etc. The actual editing UI will be built into `apps/anvil`, `apps/desktop`, and `apps/hudd` "in the not distant future," per the human — this proposal only covers the underlying mechanism, not any specific app's editor UI.

## The critical architectural decision (confirmed by the human, 2026-08-15)

Edits made in edit mode are **not** just recorded in a runtime-owned database. They get applied as **precise updates to the actual source code on disk**, via an Ethos-backed parse/print engine. A runtime-side database is still needed, but *only* for in-flight changes (undo, live preview before commit) — source-on-disk is the canonical target, not a cache or log of edits that lives separately from the code that produced the UI in the first place.

This is *why* edit mode hasn't been built yet, even though part of its prerequisite (stable node identity, needed for selection) already exists in the framework — see "What already exists" below. The real blocker is the Ethos pipeline maturing, not anything in Escher's own runtimes.

## What already exists (from this session's other work, reusable here)

- **Selection is close to free.** `escher_core::scaffold::NodePath` already gives every node stable identity across draws (`(TypeId, usize)` key chain) — "what's selected" is naturally "the current `NodePath`."
- **Hit-testing has a gap, not a hole.** Surfaces already compute per-node bounds during layout (`AppKitSurface::layout_children` computes an `NSRect` per node today) but discard that information once native objects are positioned. Edit mode needs a persisted `NodePath -> bounds` map per draw so a click can resolve back to a node — a real addition, but a small one layered on top of work that's already there.

## Blocker: the Ethos side isn't built yet

Grounding checked 2026-08-15 (see `[[project_escher_ethos_multiruntime_2026_08]]` for the fuller, mostly-unverified Ethos session history this pulls from): `ethos/dialects/ecma` already has the right *shape* of infrastructure for precise source mutation — a real `swc_core`-backed parser/printer with whitespace/comment-preserving round-trip, built behind a `Dialect` trait, on the explicit stated principle "a dialect turns source into an AST and prints it back; a runtime should only execute code." That's the right foundation *pattern*.

But:

- **There is no Rust-targeting dialect.** Everything built in Ethos so far targets ECMAScript/JS. Escher app source (the `draw_fn` closures this whole framework is built around) is plain Rust — a new dialect implementation would be needed before any precise Rust-source edit could round-trip through this pipeline.
- **No connection found between Ethos's dialect system and this workspace's own `escher-syn`/`escher-jsx` packages.** These exist in `escher`'s own `packages/` directory and sound relevant by name, but nothing in either the code or prior session memory ties them to Ethos's parse/print machinery. Don't assume they already solve this — verify before building against that assumption.

## Recommended next step

Not implementation — a **scoping proposal for the Ethos-side Rust dialect** is the actual next concrete step, and belongs as Ethos's own spec artifact (outside this repo, in whatever `spec/`-equivalent Ethos itself uses), not here. This document exists so a future Escher-side session doesn't attempt to build edit mode against a runtime-only database, or assume `escher-syn`/`escher-jsx` already provide the missing piece, without first confirming the Ethos Rust dialect exists.

## Explicitly out of scope until the blocker clears

- Any actual editor UI in `apps/anvil`/`apps/desktop`/`apps/hudd`.
- The per-runtime "thin layer" (selection/insertion/deletion interaction handling) itself — while the selection half is nearly free given `NodePath`, insertion/deletion have no real target to write to until the Ethos Rust dialect exists, so building the interaction layer first would produce edits with nowhere correct to go.
- Any runtime-side edit database/log design — deliberately deferred until the source-of-truth (Ethos) side is real, so the database's actual scope (truly just in-flight/undo state, not a permanent edit store) doesn't drift while designing it in isolation.
