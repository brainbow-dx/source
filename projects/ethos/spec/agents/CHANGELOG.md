# Agent Contribution Changelog

This file records all automated-agent modifications outside of the `./spec` folder.

Entry format (one entry per section):
- Date: YYYY-MM-DD (UTC)
- Agent: name/version
- Summary: one-line summary
- Files: bullet list of created/modified files
- Details: brief description of actions and commands run
- Issue/PR: link if created

---

Date: 2026-06-04 (UTC)
Agent: GitHub Copilot (automated assistant)
Summary: Initialized planning docs and mdBook configuration; added AI policy and changelog.
Files:
- spec/planning/refactor-project.md
- spec/planning/SUMMARY.md
- spec/planning/book.toml
- spec/planning/SUMMARY.md
- spec/AI_POLICY.md
- spec/AI_CHANGELOG.md

Details: Created planning folder and refactor project doc; added mdBook config pointing to `spec/planning` as source and configured output to `/.output/mdbook/planning`. Removed nested `book/` src and rebuilt book. Recorded AI policy and this changelog.

Issue/PR: none (created directly in repo by automated assistant per maintainer request)

---

Date: 2026-08-16 (UTC)
Agent: Claude Code (claude-sonnet-5)
Summary: Added a Scaffold→UXML/USS codegen tool (`tools/codegen/uxml/`) for Escher's Unity UI work, per repo-owner exception to `POLICY.md`'s spec-only write scope for this work stream. Confirmed live that `ethos-deno` has no TypeScript-stripping pass.
Files:
- spec/agents/proposals/uxml-uss-codegen.md (proposal, written first per POLICY.md)
- tools/codegen/uxml/mod.ts (new — reusable ScaffoldDescription → UXML/USS transform)
- tools/codegen/uxml/shape-demo.ts (new — tonight's one-off demo content, `run(args)` entry point for `ethos-cli run-command`)
- spec/agents/CHANGELOG.md (this entry)

Details: Escher (sibling project) is adding a way to render its `Scaffold` UI trees inside Unity via Unity's own UI Toolkit (UXML/USS). Per the repo owner's direction, the codegen logic lives here as ordinary TS tooling (not a new Dialect — see `spec/Dialects.md`'s precise definition, which this doesn't fit), invoked via the existing `ethos-cli run-command` path the same way Escher's Anvil app already invokes any other Ethos script. Real, confirmed gap found while building this: a `.ts` file with actual TypeScript type annotations (`const x: Foo = ...`, `interface`/`type` declarations) throws `SyntaxError: Missing initializer in const declaration` when run via `ethos-cli run-command` — `packages/deno` has no TS-erasure pass before handing the module to V8, and `ethos-ecma`'s `swc_core` parser isn't in that execution path at all. Worked around tonight by writing JSDoc-typed plain-JS content in `.ts` files; logged as a real follow-up in Escher's `spec/ROADMAP.md` (M4) since fixing it properly belongs here, not there. Verified live: `cargo run -p ethos-cli -- run-command tools/codegen/uxml/shape-demo.ts` produces the correct `{description, uxml, uss}` JSON.

The repo owner explicitly granted a POLICY.md exception for this work stream ("we're currently working through a massive refactor to prepare the product for a v1 release... make an exception for yourself until we have our fully functional Escher UI") — recorded here for auditability; the proposal doc above still exists as the design record POLICY.md asks for.

Issue/PR: none (direct commit per repo-owner's live-session exception; not yet committed as of this entry — see git status)

---

Instructions: Append new entries to this file for every subsequent AI change under `spec/`.
