# AI Agent Contribution Policy

Purpose: Ensure safe, auditable, and human-reviewed AI contributions to the Ethos repository.

Rules:
- Read-only: Agents may read all files in the repository.
- Write scope: Agents are only permitted to create or modify files under the `spec/` directory.
- No direct edits outside `spec/`: Any suggested changes to code, configs, or content outside `spec/` must be produced as a proposal inside `spec/` (design doc, patch description, or draft PR) and require explicit human approval before being committed.
- Changelog requirement: Every agent action that writes files MUST be committed cleanly with a clear indicator that the change belongs to an agent/bot.
