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

Instructions: Append new entries to this file for every subsequent AI change under `spec/`.
