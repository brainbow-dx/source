# Scaffold → UXML/USS codegen tool — proposal

Status: proposed and approved verbally by the repo owner in the same live session that requested it (2026-08-16, Escher work) — recorded here per `POLICY.md`'s requirement that code changes outside `spec/` go through a proposal first. Implementing immediately after this file lands, same session, per the owner's explicit "go ahead."

## Why this lives in Ethos, not Escher

Escher (`projects/escher`) is adding a way to render its `Scaffold` UI trees inside Unity, via Unity's own UI Toolkit (UXML markup + USS styling — Unity's own analogues of HTML/CSS). Per the repo owner's direction: "codegen tasks are for Ethos — build any code parsers/codegenerators over there and use it in Escher as needed," and separately, the short-term goal is authoring UI content in JS/TS via Ethos's `ecma`/`ethos-deno` pairing rather than hardcoding it in Rust. So the actual `ScaffoldDescription → UXML/USS` transform, and the demo content that exercises it, belong here as ordinary TS tooling — not hand-rolled Rust tree-walking code inside `escher/runtimes/unity`.

This is **not a new Dialect** in `spec/Dialects.md`'s precise sense (a dialect parses/prints a *source language*; `ScaffoldDescription` JSON isn't a language, and UXML/USS output isn't fed back through any Ethos parser). It's an ordinary `.ts` script, invoked the same way Escher's Anvil app already invokes any other Ethos script — via `apps/cli` (`ethos-cli run-command <script> <args>`, `apps/cli/src/main.rs:59-65`), which loads a module and calls its exported `run(args)`. No new Dialect/Runtime plumbing needed.

## What it does

Escher already has a JSON `ScaffoldDescription` schema for describing a `Scaffold` tree (`escher/runtimes/web/src/description.rs`) — `{ content, styles: [...], children: [...] }`, styles like `{"type":"backgroundColor","color":"#.."}`/`{"type":"size","width":{...},"height":{...}}`. Escher's web (SSG) and terminal (via a to-be-added `pub` on `apply_description`) surfaces already consume this shape. This tool adds a third consumer, in TS:

- `tools/codegen/uxml/mod.ts` — `scaffoldDescriptionToUxml(description): { uxml: string, uss: string }`. Walks the same JSON shape `description.rs` parses, emitting UXML (`<ui:VisualElement>` / `<ui:Button>` / `<ui:Label>` per node, referencing a `.uss` class per node) and USS (one rule per node, translating `backgroundColor`/`size` — the two properties tonight's demo actually uses — the same small subset `escher-terminal` and `escher-web`'s DOM surface already both handle). Reusable/importable — not one-off script content.
- `tools/codegen/uxml/shape-demo.ts` — the actual demo content for tonight: `export function run(args)` returns `JSON.stringify({ description, uxml, uss })`, where `description` is a small hand-built `ScaffoldDescription` (one colored, sized rectangle) and `uxml`/`uss` come from `scaffoldDescriptionToUxml(description)`. One script, one invocation — the same `description` drives Escher's terminal/web renderers, the same `uxml`/`uss` drive Unity, all from one `ethos-cli run-command tools/codegen/uxml/shape-demo.ts` call.

## Why one script for all three targets

The alternative — hardcoding the shape spec independently in Rust (twice) and TS (once) — was the original plan draft before the repo owner's correction. Same visual spec, one source of truth, authored where "the full stack of UI options from any language Ethos supports" is meant to live long-term, with JS/TS via `ecma`/`ethos-deno` as the explicit near-term target language.

## Scope notes / non-goals

- Not a live/hot-reload channel — one-shot, command-triggered generation, matching tonight's Escher-side MVP scope (see `escher/spec/ROADMAP.md`'s "Adjacent" section, added same session).
- `scaffoldDescriptionToUxml` only needs to cover `backgroundColor`/`size` tonight — the same subset already live in `escher-terminal` and `escher-web`'s surface, not the full `Property` set. Extending it to more properties as Escher's own surfaces grow the same coverage is natural follow-up, not blocking.
- New top-level `tools/` directory in this repo (sibling to `dialects/`/`packages/`/`apps/`/ `plugins/`/`services/`) — chosen over cramming this into `dialects/ecma` (which is parse/print only, per its own `Cargo.toml` doc comment, and is a Rust crate, not a place for a standalone TS script anyway) or `packages/deno` (the runtime itself, not tool content that runs on it). Self-contained relative imports only, so no `deno.jsonc` workspace-array change needed.
