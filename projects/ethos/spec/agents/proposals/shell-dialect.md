# Shell/bash dialect — plan (not started)

Status: proposed, 2026-08-15. Written by an agent fork per human request, for the root agent to
pick up and coordinate later. Not started — no code, no crate scaffolding.

## Goal

The smallest support for a genuinely *working* bash-ish interpreter, usable as an embedded
scripting engine inside Brainbow apps — not full POSIX conformance, but not a toy subset either.
Concretely: variables, pipes, redirection, command substitution, functions, control flow
(`if`/`for`/`while`), enough expansion/quoting to run real utility scripts without surprises.

## Grounding: how Ethos dialects actually work today

Verified live against the repo, not assumed:

- `ethos_core::Dialect` / `ethos_core::Runtime` (`packages/core/src/lib.rs`) are minimal traits:
  `Dialect::parse`/`print`, `Runtime::execute(&mut self, source: &str) -> Result<String, E>`.
  Very little boilerplate either way.
- `dialects/ecma` (102 lines) and `dialects/lua` (118 lines) are thin wrapper crates around a
  mature existing engine (`swc_core`, `mlua`) — cheap, because the hard part (parsing/executing
  the language) is someone else's problem.
- `dialects/c` (615 lines) is hand-rolled from scratch (lexer/parser/interpreter), and says so in
  its own header: a deliberately reduced toy subset, "not a conformant C implementation."
- No shell/bash crate is currently vendored anywhere in the workspace (`ethos/Cargo.toml`, root
  Brainbow `Cargo.toml` — no `brush`/`conch`/`yash` hits as of this writing).
- See `spec/Dialects.md` for the full current-dialects table and the original LLVM-codegen vision
  this is a pragmatic step short of (same relationship `ethos-lua`/`ethos-c` already have to it).

## Two paths, and why one is strongly preferred

**Path A — wrap an existing embeddable Rust bash engine** (same shape as `ethos-ecma`/`ethos-lua`).
Candidate: `brush-core` (a maintained POSIX/bash-compatible shell implementation in Rust,
published to be usable as a library, not just a binary) — **unconfirmed by this agent**, no
network access available to verify its current crates.io state, license, or embeddable API
surface. **First real task for whoever picks this up: confirm that before anything else** — it's
the fact that separates a multi-day estimate from a multi-week one.
- Estimate if confirmed usable: ~1–3 days (agent-session time, not calendar time) for the wrapper
  crate, `Dialect`/`Runtime` impl, builtin/PATH-resolution decisions, and real-script test
  coverage.

**Path B — hand-roll from scratch**, `ethos-c`-style. Given the "usable scripting engine" bar
(not the C-dialect's toy-subset bar), this is a much bigger lift than 615 lines: bash's
quoting/expansion rules are most of the real complexity, and there's no shortcut around them by
hand. Estimate: on the order of a couple weeks of agent-session time, not the few days Path A
would take.

**Recommendation: Path A**, contingent on the `brush-core` (or equivalent) verification above.
Only fall back to Path B if no suitable embeddable crate actually exists or its license/API rules
it out.

## Scope notes

- Out of scope for a first version: job control, interactive-shell features (line editing,
  history), full POSIX corner cases. This is an embedded scripting engine for app automation, not
  a login-shell replacement.
- Pairing: per `spec/Dialects.md`'s existing pattern, this would likely be `ethos-sh` (or similar)
  pairing with itself as both dialect and runtime, the same way `ethos-lua` does — no separate
  parse/print step needed unless a real use case for round-tripping shell source shows up.

## Open questions for the root agent

1. Confirm `brush-core` (or find a better-fitting alternative) is actually embeddable as a library
   with an acceptable license, before scaffolding anything.
2. Decide the crate name/pairing convention (`ethos-sh`? `ethos-bash`?) and whether it belongs in
   `dialects/` alongside `ecma`/`lua`/`c`, matching existing layout.
3. Decide how deep "usable as a scripting engine for Brainbow apps" needs to go for v1 — the
   feature list above (variables/pipes/redirection/substitution/functions/control-flow) is this
   agent's best guess at "smallest working set," not a human-confirmed spec.
