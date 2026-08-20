# PR #1 review cleanup plan

Tracking doc for working through all 57 pending comments on `brainbow-dx/source#1` (`escher/mario-platform-polish` → `main`), review 4983828003. Update checkboxes as items land; don't delete finished items, so a fresh session can see what's already done versus what's still open. New comment batches from the user get appended as new phases, not folded into these four — see `spec/.agents/handoff.md` for the current live status.

## Phase 1 — `.output/` sweep (repo-wide)

Comment: "ALL build artifacts, logs, etc should be placed in `.output` .. Need a repo-wide sweep before we continue review" (`atlas/docs/book.toml`, id 3823579689). Blocking — done before anything else.

- [x] Audit every project for where build artifacts/logs currently land (mdbook's `dist/public`, cargo `target/`, deno `.output/`, ad hoc log files, etc.) and confirm the intended convention: everything non-source-controlled that a build/run produces goes under a project's own `.output/`.
- [x] Update `book.toml` `[build] build-dir` across all 6 docs projects + the template to `.output/...` instead of `dist/public`.
- [x] Check Dockerfiles/compose files for hardcoded paths that assume the old location.
- [x] Update `.gitignore`s to match (`.output/` ignored, not the old scattered patterns).
- [x] Verify with a real `mdbook build` per project again after the path change.

## Phase 2 — Real command host API (sentinel removal)

Comments: quit.js (3822618136), open-page.js commands/scripts split (3822676057), and repeated emphatic main.rs comments (3823242595, 3823262356, 3823383244) — commands must be fully self-contained, auto-discovered, and run entirely in the scripting layer, with **zero** Rust-side interpretation of their output. This supersedes the earlier "just write a proposal" call from before this review round — the user wants it actually built now.

- [x] Design the host-callable op surface — landed as one generic `op_host_action` carrying a real `{ type, ...data }` message, not per-action ops (`op_anvil_quit`, etc.) — see `spec/.agents/proposals/anvil-command-host-api.md` for why generic won over specialized. `ethos-deno`'s new `host_actions` module.
- [x] Design the cross-thread handoff from the Deno runtime thread back to `AppState`/the Bevy main loop — reused the existing `quit_requested: Arc<AtomicBool>` pattern `AppState` already had, just given a real trigger instead of a sentinel-string match.
- [x] Wire the extension into wherever Anvil constructs its command-running Deno runtime (`run_module_command`'s new `extensions: Vec<Extension>` param, threaded through `process::run_js_command`).
- [x] Migrate `commands/quit.js` and `commands/clear.js` off `QUIT_SENTINEL`/`CLEAR_SENTINEL` onto the real op (via a `postMessage`/`globalThis.__ethosHostAction` wrapper); sentinel constants and their Rust-side string-matching are gone.
- [ ] Live-verify both commands actually still work end to end — `ethos-deno`'s own `host_actions.rs` test drives the real op mechanism through a real script and passes; `cargo check` is clean for `ethos-deno`, `ethos`, and `escher-anvil`. A real tmux run of `escher-anvil` itself (`/quit`, `/clear`) hasn't happened yet — deferred given today's repeated near-full-disk incidents from full binary link builds; do this before calling Phase 2 fully closed.
- [ ] Revisit `/relay-console`: still Rust-hardcoded today (per the earlier proposal doc) — decide whether it becomes a real discovered command on top of this same API or stays a deliberate, documented exception (it manages a long-lived subprocess, not a fire-and-forget action).

## Phase 3 — Repo-wide mechanical rules batch

- [ ] Deps declared on a single line even when long (`atlas/packages/core/Cargo.toml` flagged specifically, id 3823600649) — sweep all `Cargo.toml`s, log the rule in `principles.md`.
- [ ] No emoji in technical tooling output — sweep (`QUIT_SENTINEL`/`CLEAR_SENTINEL` overlap with Phase 2; check elsewhere too), log in `principles.md`.
- [ ] `default_value` over `default_value_t` for clap args — sweep all clap definitions, log in `principles.md`.
- [ ] Resume the panic audit interrupted earlier this session: `Result`/`?` instead of panicking in runtime code; panics only acceptable at startup/setup — log that boundary explicitly in `principles.md`.
- [ ] Idiomatic imports / direct type use at call sites — apply where flagged and sweep nearby code for the same pattern.
- [ ] Inline SQL string formatting: extra indent level inside the string itself so the delimiters read as a visual block (ids 3823831195, 3823991747) — apply to every inline SQL call, not just the flagged one.
- [ ] Consistent `params!` macro usage even for empty params (id 3824023111).
- [ ] Readable multi-line formatting for struct construction that skips a ctor fn (id 3824218321).
- [ ] Always use `crate::log::init(..)` instead of ad hoc tracing setup (id 3823595719, `atlas/packages/relay/examples/serve.rs`) — check other examples for the same gap.

## Phase 4 — `apps/anvil/src/main.rs` decomposition

The largest phase — ~35 comments, almost all on this one file.

- [ ] Split chat logic into its own module (id 3823216312).
- [ ] Split task logic into its own module (id 3823222068).
- [ ] Split command logic/types into their own module (id 3823342960).
- [ ] Move the "assistant terminal plugin" out of `main.rs` into the terminal runtime as reusable helpers (id 3823049451).
- [ ] Fix the "wild Claudeism" (id 3823079518): Anvil currently *replaces* the internal Bevy terminal plugin instead of *using* it. Needs real investigation into why it was built that way before just swapping it back — check history/`changelog.md` for why the replacement happened in case there was a real reason.
- [ ] Theme/styleguide application shouldn't be toolbar-specific — apply to all scaffold-styled components generally; move styleguide color utilities into their own module (id 3823199277).
- [ ] Ports/paths (relay port, relay console port, script paths) configurable via `.anvil.toml`, not hardcoded (ids 3823310367, 3823322098) — get the path from config relative to `.anvil.toml` itself, not a hardcoded literal.
- [ ] Drop the helper judged "useless" — use `Styleguide` directly (ids 3823328101, 3823329500).
- [ ] Long trace-config lines → constants in `config.rs` (id 3822991457).
- [ ] Arg defs one-per-line where possible (id 3822994508).
- [ ] Trim the overly-verbose comment near the clap help text (id 3823013515) — brief comment for the help page is enough.
- [ ] Make sure every clap parser has a real, useful `--help` (id 3823008014).
- [ ] Sharpen one overly generic name (id 3823294520) — find the exact identifier from the comment's diff context.
- [ ] Move docker utilities out of `config.rs` into Atlas, exposed as infra/cloud/network utils (id 3822975376) — cross-project move, coordinate with Phase 2/3 changes touching the same files if timing overlaps.
- [ ] Move the small "shouldn't be in main" helper(s) somewhere organized (id 3822984345, 3823219807) — best judgement on exact location per the comment.
- [ ] Log the two standalone "mark this in principals" asides (ids 3822999636 already covered above, 3823245229 — re-read that comment's context to see which specific rule it's tagging) in `principles.md`.
- [ ] Note-only items — do **not** implement now, just log in `spec/ROADMAP.md` or a proposal doc: page system should become dynamic/extension-configurable (id 3823378045); two "this isn't quite right, revisit" flags (ids 3823299533, 3823348097).

## Notes

- Phases aren't strictly sequential where they don't depend on each other — if Phase 3/4 items unblock faster once Phase 1/2 land, do them out of order rather than waiting.
- Every phase gets real verification (`cargo check`/`cargo test` at minimum, live smoke test where behavior actually changes) before being marked done — no exceptions, per this session's own build.rs regression lesson (see `handoff.md`).
- More comment batches are still coming from the user in "manageable chunks" — this doc covers the 57 comments read as of 2026-08-20. A new batch gets its own new phase(s) appended here, not merged into these four.
