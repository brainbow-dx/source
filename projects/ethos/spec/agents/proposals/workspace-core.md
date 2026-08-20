# Ethos `Workspace` — a shared, embeddable core (design capture, not started)

Status: vision recovered and written up 2026-08-17, at the human's explicit request, after CLI
work across the monorepo drifted out of sync with earlier decisions and some of it needed
re-deriving from scratch. No code, no crate scaffolding — this document exists to stop the vision
from getting lost a second time, not to lock in a design the human hasn't actually settled yet.

## Where this came from

`spec/Project.md` (written directly by the human, 2026-08-10, one commit, never touched since)
named three tools alongside the four project pillars: **Sketch** (UI/UX prototyping — since built,
inside Escher), **HUD** (quick tool access — not built, and easily confused with Escher's unrelated
`apps/hudd` overlay daemon, different concept, similar name), and **Bench** (workspace management —
never mentioned again anywhere in the repo by that name).

Separately, a 2026-08-15 Escher session independently re-derived a related need and wrote it up in
`projects/escher/spec/.agents/handoff.md` as "the Ethos `Workspace`/nested-CLI vision" — `ethos
<monitor|dev|build|codegen>` typed inside Anvil, `escher anvil` as the outer launcher — with no
cross-reference to `Project.md` or "Bench" at all. That file is explicitly overwritten every
session, so until now that was the *only* surviving copy of the idea.

Asked directly, the human doesn't recognize "Bench" as something they specified — it may be an
earlier agent's invention that made it into `Project.md` without being a real, discussed decision.
**Don't treat the name "Bench" as meaningful.** What *is* real, confirmed directly by the human
2026-08-17: the underlying need for shared, embeddable workspace-management logic, described in
their own words below. Where exactly it should be built and how far it should reach hasn't been
decided — that's the open part of this document, not the part being asserted as settled.

## The ask, in the human's own words

> I like the idea of workspace management, but that's Ethos' job. It will eventually be home to a
> central "Workspace" data structure instance per CLI run/workspace instance (an LSP, the Anvil
> CLI, and a web worker might all want to run the workspace from their respective environments).
> So, it should be lightweight, but it will be the central point at which all other Ethos actions
> orchestrate code-based workspace mutations, and how all other projects will interact with
> Brainbow projects together. For example, an Anvil script, a Rust program core to run in a Unity
> game, and a web worker meant to control the cache behavior of a PWA app might all also have good
> reason to reason about the makeup of a Brainbow-based project. So, I want to be able to embed it
> anywhere, and we don't know yet exactly where.

Breaking that down into what's actually being asked for:

- **One `Workspace` type, instantiated per environment, not one global daemon.** An LSP process, an
  Anvil CLI invocation, and a web worker each hold their own `Workspace` instance rather than all
  talking to one shared running service — "instance per CLI run/workspace instance," not
  "singleton server."
- **Ethos owns it.** Confirmed explicitly — if "Bench" turns out to mean anything real, it's Ethos's,
  matching `Project.md`'s original pillar assignment (Ethos = Codegen), not Escher's, even though
  the closest prior *discussion* of this happened in an Escher doc.
- **It's the orchestration point for code-based workspace mutations.** Not a passive read-only
  model of "what files exist" — the thing other Ethos actions (dialects, codegen, whatever else)
  route mutations through.
- **It needs to be embeddable in genuinely different runtime environments**, named explicitly:
  an LSP, the Anvil CLI (native Rust/Bevy), a Rust program compiled into a Unity game (Aby), and a
  browser web worker (controlling a PWA's cache behavior). That's at least four different
  compilation/execution targets — native Rust binary, native Rust-in-game-engine, and
  Rust-or-JS-in-browser-wasm — which is a real constraint on how it can be built, not a detail to
  defer.
- **"Lightweight" is explicit and load-bearing.** Whatever this becomes, it shouldn't force any of
  those four environments to carry more weight than they need just to hold an instance of it.

## What's explicitly *not* this, so it doesn't get re-blended

- **Not a nested CLI shell inside Anvil.** The human separately chose "per-project CLIs, shared
  convention" over "Anvil hosts a shell for other CLIs" when scoping this round's CLI work — the
  2026-08-15 "type `ethos monitor` inside Anvil" framing is superseded by that choice. `Workspace`
  may still end up *used by* Anvil's CLI (it's one of the four named embedding targets), just not
  via a nested-shell UI.
- **Not a unified top-level `brainbow` command.** Also explicitly ruled out in the same scoping
  conversation. `Workspace` is a library/data-structure concept, not a CLI surface of its own.
- **Not Escher's own CLI work.** A real `clap`-based `escher` CLI (`escher anvil`, cargo-style) is a
  separate, already-decided, already-blocking-other-work item tracked in Escher's own
  `spec/ROADMAP.md` (M6) — sibling effort, not this one, though `escher`'s CLI is plausibly a future
  *consumer* of `Workspace` once both exist.
- **Not "HUD"** (`Project.md`'s third named tool, "quick tool access") — a different, still-unbuilt
  concept, easily confused with Escher's unrelated `apps/hudd` overlay daemon. Worth its own
  disambiguation pass at some point, out of scope here.

## What already exists that's adjacent, worth checking before building anything new

- `projects/ethos/services/dev` (`@ethos/dev`) is real, working, shared Deno tooling — already
  imported by multiple projects' own dev scripts (`shell.ts`'s `$`/`ManagedProcess` helpers). Not
  the same shape as what's being asked for here (that's process-spawning/shell utilities, not a
  workspace-mutation data structure), but it's the closest existing "shared cross-project Ethos
  tooling" precedent, and whatever `Workspace` becomes should probably not duplicate it.
- Ethos's `Dialect`/`Runtime` trait split (`packages/core`) is the existing extension point for
  "how does Ethos act on code" — `Workspace` as "the central point other Ethos actions orchestrate
  mutations through" likely sits adjacent to or above this, not inside it. Worth confirming that
  relationship explicitly before writing a real design, since it changes where `Workspace` would
  live in the crate graph.

## Open questions — genuinely unresolved, not this document's to answer

The human was explicit that "we don't know yet exactly where it should be embedded" — these are
the real decisions still needed before any code gets written, not gaps in this write-up:

1. **What does a `Workspace` instance actually expose on day one?** A read-only model of "what
   Brainbow projects/files exist and how they relate," a mutation API, both? The four named
   consumers (LSP, Anvil CLI, Unity-embedded Rust core, PWA web worker) have very different needs —
   an LSP wants rich semantic queries, a cache-control web worker probably wants almost nothing by
   comparison. Worth asking directly: what's the smallest useful slice that's still worth building
   first, and which of the four consumers proves it out?
2. **What does "embed anywhere" actually require mechanically?** A plain portable Rust
   crate compiled natively into some consumers and to wasm for browser/web-worker use is the most
   obvious shape given the four targets named, but that's an inference, not a confirmed decision —
   worth checking explicitly rather than assuming, especially since "a web worker" and "a Rust
   program core in a Unity game" have very different FFI/build stories even if both compile from
   the same source.
3. **Which consumer gets built first, as the proof case?** Building all four integration points at
   once isn't realistic for a first slice. Given Escher's Anvil CLI is the most actively-developed
   consumer of Ethos today (`/shape`, `ethos-cli run-command`), it's a plausible first target, but
   that's a suggestion for discussion, not a decision made here.

## Immediate next step

Nothing to build yet. Next real step is a short conversation with the human to resolve question 1
above (the minimal day-one surface) — everything else in this document should hold up regardless
of how that lands.

## Decided, 2026-08-19 — question 1-3 resolved, first code landed

Asked directly, the human resolved all three open questions above:
1. **Day-one surface**: read-only — "what Brainbow projects exist," not a mutation API yet.
2. **First consumer**: Anvil's CLI (Escher), matching this document's own suggestion.
3. **Embed mechanism**: wasm-compilable from day one, not deferred — filesystem access lives
   behind a `WorkspaceFs` trait so the core type has no direct dependency on one existing.

New crate `packages/workspace` (`ethos-workspace`). `Workspace::scan` walks a root's `projects/` directory and tags each project `rust`/`deno`/`node` by marker-file presence (`Cargo.toml`, `deno.json(c)`, `package.json`) — nothing deeper (no file trees, no dependency graph between projects) on purpose, per question 1's answer. `NativeFs` (real `std::fs`) sits behind a `native-fs` feature, off by default, so a default build compiles for `wasm32-unknown-unknown` — confirmed live with `cargo check --target wasm32-unknown-unknown`. Verified live against this actual monorepo: `Workspace::scan` correctly found and tagged all 6 real `projects/*` entries. Anvil's new `/workspace` command (`apps/anvil/src/main.rs`'s `describe_workspace`) is the first embedding — lists whatever the scan finds under `anvil_root()`.

Separately surfaced and explicitly *not* folded into this: `Brainbow.ethos` and `projects/ethos/examples/config.ethos` use a real `workspace { ... }` keyword, but that's a much older, separate idea (per the human directly) — a declarative config *language* (a real dialect, originally imagined with its own small LLVM-IR interpreter via `inkwell`) for Escher/Eden/Atlas etc. to describe how the pieces of the system fit together, not this document's embeddable Rust data structure. Worth its own proposal if it's ever picked back up; the naming collision is coincidental, not a sign the two should merge.
