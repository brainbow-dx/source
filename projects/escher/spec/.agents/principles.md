# Working principles

Not process for its own sake — every rule below cost real time (and, for an AI agent, real tokens) on a specific night (2026-08-16) when it was skipped. Read this before diving into a non-trivial change; it's cheaper than re-learning any one of these live. Linked from `AGENTS.md`.

## Maintaining project health, not just shipping the feature

**Any serious feature that needs ongoing monitoring or housekeeping should leave behind a real, reproducible way to check it — not necessarily unit tests.** What "real" means varies by what the feature actually is: a small integration test for a pipeline with real inputs/outputs (see `apps/anvil/src/shape.rs`'s test module — a JSON fixture captured from a real script run, not a hand-simplified stand-in, used across several tests so they can't quietly drift from what the app actually sends); an example committed to run as part of the normal dev workflow, re-run by hand or in CI, not written once and forgotten; OpenAPI/Bruno-style docs in `spec/` for a human to drive API testing directly, when the thing being built is a service with a real API surface rather than a library. Pick whichever shape actually fits — the point is a repeatable, low-friction way to answer "is this still working" later, not a specific test framework. Watch for chances to add this as you go, not just when asked — a feature that's genuinely done includes knowing how you'd notice if it broke.

## Before touching code

**Read this project's own `AGENTS.md`/`POLICY.md` first, every session, not just the first time.** Escher's `AGENTS.md` (like Ethos's `spec/agents/POLICY.md`) defaults to "agents propose in `spec/.agents/`, humans implement" — not "agents write code freely." A human can override that for a session, but the override should be explicit and stated, not something that quietly becomes the norm mid-session because it worked the first three times. If you're about to write code directly, name the reason (an explicit ask, an explicit exception) rather than assuming it still applies from earlier in the conversation.

**Search broadly before claiming something doesn't exist.** A whole architecture pivot got proposed tonight on the premise that `@ethos/dev` — a package several existing scripts already imported — didn't exist. It did: fully built, working, registered in Ethos's own workspace, just under `services/dev`, not `packages/dev`, which is the only place that got checked. "I grepped one plausible directory and found nothing" is not the same claim as "this doesn't exist" — in a monorepo with `packages/`, `services/`, `apps/`, `plugins/`, and `dialects/` all in active use, check the shape the *rest of the project* actually uses before concluding something's missing. Prefer running the thing (or trying to import it) over grep alone — a real failure tells you *why* something doesn't resolve, which a missing grep hit never does.

**Check for existing tooling with the same job before building new tooling.** Tonight nearly invented a fresh JSX-to-Scaffold authoring mechanism before discovering `packages/jsx` (`@escher/jsx`) already did exactly that, complete and working, one `deno run` away from proof. Search for prior art in the monorepo by what it *does*, not just by the name you'd give a new thing — a differently-named package solving the same problem is a very easy miss.

## Building interactive apps (Anvil, and anything like it)

**Never block the render/input thread on I/O, even "briefly."** A 750ms timeout on a persistence write still reads as "the UI stalled" under real interactive use (rapid input, continuous dragging) — perceptual responsiveness thresholds are far tighter than "bounded and not literally infinite." The fix that actually holds up: push UI state first, persist/sync in the background, always, with no exception for "this one call is supposed to be fast." If a call can ever touch a network or a slow resource, it does not belong on the thread the user's input has to pass through.

**One shared connection to an external resource needs one owner, not N concurrent callers.** Fixing "don't block the render thread" by spawning N independent background tasks — one per input — still leaves all N racing into the same `sqld` connection concurrently, with no verified guarantee the client handles that safely. Route writes through a single queue drained by one task instead. This is a "no regret" fix regardless of whether the underlying client actually needed it — it costs almost nothing and removes the question entirely.

**Be selective about what actually needs to survive a crash.** Not everything that gets persisted needs to be written the instant it changes. Chat messages and explicit user actions are real data — persist those promptly, in order. Cosmetic, high-frequency, transient state (a window's drag position) is not — debounce it, and when a burst of updates queues up anyway, only the *last one* ever needs to actually reach disk. Treat "should this write happen at all, right now" as a real design question, not something that falls out automatically from "we have a queue now."

## Writing comments and error messages

**A comment or error message is read by someone with none of today's session in their head — write it that way, every time.** Several doc comments and a test failure message ended up narrating the session that produced them instead of stating the underlying fact: a specific date, "confirmed live," a phrase like "per the user directly" attributing a decision to an out-of-band conversation the reader was never part of. None of that is information a cold reader can use — "per the user directly" doesn't tell them *why* a design choice is deliberate, just that someone once said so; a date doesn't help someone debugging the same failure a year later. State the underlying fact or constraint plainly instead ("libsql's fork doesn't implement this" beats "confirmed 2026-08-19 that libsql's fork doesn't implement this"; "deliberate — see the design doc" beats "per the user directly"). Session narration, dates, and attribution belong in `spec/.agents/changelog.md`, which exists exactly for that, never in the code itself.

## Naming things

**Balance brevity with readability — a name shouldn't try to be the whole spec.** A test once ended up named `two_peers_discover_each_other_and_relay_a_handshake`, narrating the entire scenario instead of naming it (`two_peers_relay_a_handshake` says the same thing a reader needs at a glance). Multiple words are fine — `parses_colors_and_dimensions` is clear and still short — but once a name is restating steps or listing every precondition, that detail belongs in the body (a doc comment, a `//` above the interesting line) rather than getting packed into the identifier every future caller has to read and type in full.

**Keep a single log/tracing statement on its own one line, even when that line runs long.** A
`tracing::debug!`/`info!`/etc. call split across several lines by rustfmt-style wrapping is harder
to grep for and to read as one unit than the same call on one line — the same "let it run long"
reasoning as the Markdown rule below, applied to a single statement rather than a whole paragraph.

## Writing Markdown

**Don't hand-wrap prose at some fixed column — let lines run long.** Markdown parsers and renderers generally treat line breaks in the source as meaningful (a hard break, or at least a paragraph/line boundary depending on the renderer), not as pure formatting whitespace to be reflowed. Manually breaking a paragraph every ~80-100 columns bakes one specific width's worth of formatting into the content itself, and fights whatever width the actual renderer (a doc site, a terminal, an editor pane) wants to use. Write each paragraph as a single long line and let the reader's own tool soft-wrap it for display — VS Code's `editor.wordWrap` (already set to `"on"` for Markdown in this repo's `.vscode/settings.json`, the root workspace's, and the project template's) exists for exactly this.

## Anvil's `commands/` vs `scripts/` split

**`commands/` holds real anvil commands — scripts a user invokes with `/name`, loaded and
registered by `discover_js_commands`. `scripts/` holds user-facing dev scripts, not internal
implementation helpers a Rust-side command handler happens to shell out to.** These got split
oddly in one pass: `scripts/open-page.js` and `scripts/serve-relay-console.ts` are pure
implementation detail of the (Rust-hardcoded) `/relay-console` command, not scripts any user runs
directly, so neither bucket actually fits them today. See
`spec/.agents/proposals/anvil-command-host-api.md` for the real fix under discussion (making
`/relay-console` a proper discovered command like everything else) — until that lands, don't add a
third undefined bucket; ask before placing a new Rust-invoked helper script anywhere.

## Crossing a process or language boundary

**Verify the actual data contract by reading the producing code, not by inferring it from a doc comment.** Getting `ethos-cli run-command`'s stdout framing right (progress output interleaved live, the real payload always the *last* line, printed via `print!` with no trailing newline) took actually reading `apps/cli/src/main.rs`'s `main()`, not just its `RunCommand` doc comment. Doc comments describe intent; only the code confirms what a byte stream actually looks like.

**Prefer "any tool can use it" contracts over "you must use this specific channel."** Making progress-reporting depend on scripts remembering to use `console.error` instead of `console.log` (so it wouldn't collide with a payload-parsing convention) was solvable, but the better fix was changing the *parsing side* to only look at the last line — so script authors never have to think about the distinction at all. When a constraint would require every future caller to remember a rule, look harder for a fix that removes the rule instead.

## Working alongside a human's live session

**Never rebuild or relink a binary someone might currently be running.** On Apple Silicon, macOS kills a running process the instant the file backing its executable pages changes — this isn't a race condition, it's guaranteed. `cargo build`/`cargo run`/`cargo test` on a binary crate all relink the output binary; `cargo check` alone does not. Use an isolated `CARGO_TARGET_DIR` for an agent's own verification builds in any project where a human might have their own long-running instance going, from the *first* build of a session, not after the first killed process is reported.

**State verification status precisely, every time, not just for the parts that failed.** "Built and compiles" is not "ran and confirmed correct" is not "the exact real thing the user will see." Say which one is true for each specific piece of a change, especially once a change has several pieces at different confidence levels — confidence from a verified piece should never bleed onto an adjacent unverified one just because they landed in the same commit.
