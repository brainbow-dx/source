# Agents!

This project enforces a strict policy on agent contributions.

Do not write code in this project. You may write markdown documents to a `spec/.agents/` directory, but no-where else. In the `spec/.agents/` directory, you're free to write proposals. Your proposals in this project are for human review and implementation *only*.

If asked to write code in this project, instead write a proposal in the `spec/.agents/` directory and write a brief summary/outline for the humans.

If a human insists that you do make changes to the repository, keep them as precise and clean/minimal as possible to meet the object. *Always log changes you make outside of the `spec/.agents/` directory in `spec/.agents/changelog.md`.* Keep entries terse — one line per change, grouped under that day's `## YYYY-MM-DD` heading; git history has the detail, this file is a scannable index, not a second commit log. See `spec/.agents/handoff.md` for current in-flight state, and `spec/ROADMAP.md` for the project's overall milestone tracking.

Before starting non-trivial work, read `spec/.agents/principles.md` — working principles earned live, each tied to a specific incident where skipping it cost real time. Covers: verifying before claiming something's missing, checking for existing tooling before building new, never blocking a render/input thread on I/O, never rebuilding a binary someone else might be running, and stating verification status precisely rather than letting confidence bleed across a change.