# Escher MCP server

Status: vision only, proposed 2026-08-18. Not started. Blocked on Eden (local-inference/tool-
calling SDK, see `project-eden-vision`) being built out enough to host this, so not actionable yet.
Captured here so the direction isn't lost before then.

## The goal

Let Claude (or any other agent) inspect the surface a person is currently using: see its live
state, then cross-reference that against backend data, without disrupting the app itself. Read-only
introspection, not another input source competing with the person actually using the app.

## Three separate problems, not one

1. **Cheap structured snapshot.** Anvil's `/inspect` (turns, fps, persistence target, session dir)
   is already the right shape of data, just trapped inside the TUI with no way to reach it from
   outside. Generalizing this means each Escher app exposing a small read-only introspection
   surface (active page/view, session id, backend pointers) an MCP server can query without
   touching the render loop.
2. **Visual capture, on-screen or not.** A real OS-level problem, separate from #1: capturing a
   window's actual pixels even when occluded or minimized needs real OS APIs (on macOS, something
   like `CGWindowListCreateImage` or ScreenCaptureKit, each with its own permission model), not
   something Escher's terminal/Bevy rendering can provide on its own. Likely belongs in
   `runtimes/os`/the AppKit runtime as "give me a bitmap of window/process N" plus a way to report
   PIDs/window handles, kept strictly separate from an app's own event loop so querying it never
   disrupts anything the user is doing.
3. **Safe SQL over `sqld`.** A raw libsql connection handed to an agent is arbitrary-SQL access, a
   different risk profile than a read-only, schema-aware query gateway. This gets easier once one
   shared place knows every app's schema, rather than the MCP server needing bespoke knowledge of
   each app's own tables (Anvil's chat/tasks, Mario's gamepad-sightings/ghosts, whatever comes
   next) — the same gap the Atlas Store direction is meant to close. Worth sequencing this proposal
   after Atlas Store exists, not before. A real option already exists rather than needing to be
   built: Hasura's Turso/libSQL connector introspects a database's schema and generates an instant
   GraphQL API (queries and mutations) on top of it, a genuinely safer, schema-aware surface than
   handing an agent a raw connection. Worth evaluating directly against this instead of hand-rolling
   an equivalent.

## Explicitly not decided yet

Where the MCP server itself runs (one process per Escher app instance, or one shared daemon
querying all of them), how #2's OS permission prompts get handled non-interactively, and the exact
tool surface (one tool per concern above, or a unified "describe current state" call). None of this
needs resolving until Eden's own tool-calling story is further along.
