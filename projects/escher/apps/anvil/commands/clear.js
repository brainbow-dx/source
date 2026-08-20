// A command script exports an async `run()` returning the reply text to show (or "" to show
// nothing, see below).
//
// Wipes the assistant's messages and tasks tables. Talks to sqld directly over its HTTP API
// (the same instance assistant.rs's Rust-side Persistence connects to) since this script has no
// access to the running process's own libsql connection.
//
// Calls the real host "clear" action on success (see `ethos_deno::host_actions` and
// `spec/.agents/proposals/anvil-command-host-api.md`) instead of returning a sentinel value for
// Rust to grep the reply text for — `spawn_js_command` reacts to that real action by wiping the
// live, in-memory transcript too (not just `sqld`'s rows), then returns without ever recording a
// reply, so a `/clear` that had just logged its own "js(clear)" line right after clearing
// everything doesn't leave that line behind, the opposite of what "clear" means. Returns "" on
// success accordingly (never actually shown). A real failure below still returns actual text, so
// genuine errors still show up normally.
//
// Needs `fetch()` to actually work inside the embedded JS engine — see `ethos-deno`'s `build.rs`
// (`RESIDUAL_LAZY_ESM_SOURCES`) for the real, separate bug that broke this until it was fixed:
// the embedded runtime's V8 snapshot didn't include `deno_fetch`'s lazy-loaded ESM sources, so
// any embedded command calling `fetch()` — this one included — failed outright with "cannot be
// lazy-loaded as it was not included in the binary."

// `postMessage(message)` is a thin, deliberately-familiar wrapper around the real host action op
// — same public shape a script author already knows from `Worker`/`BroadcastChannel` messaging,
// even though the actual mechanism underneath is a direct host call (see `commands/quit.js` for
// the fuller explanation of why). The whole message survives the round trip, not just a
// squeezed-out type name.
function postMessage(message) {
  if (typeof message === "string") message = { type: message };
  if (!message?.type) throw new TypeError("postMessage(message) needs a `type` field");
  globalThis.__ethosHostAction(message);
}

const SQLD_URL = "http://localhost:8081";

export async function run() {
  let response;
  try {
    response = await fetch(`${SQLD_URL}/v2/pipeline`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        requests: [
          { type: "execute", stmt: { sql: "DELETE FROM messages" } },
          { type: "execute", stmt: { sql: "DELETE FROM tasks" } },
          { type: "close" },
        ],
      }),
    });
  } catch (error) {
    return `Could not reach sqld at ${SQLD_URL}: ${error.message}`;
  }

  if (!response.ok) {
    return `sqld at ${SQLD_URL} returned ${response.status} ${response.statusText}`;
  }

  const body = await response.json();
  const errors = body.results.filter((result) => result.type === "error");

  if (errors.length > 0) {
    return `Failed to clear data: ${errors.map((error) => error.error.message).join(", ")}`;
  }

  postMessage({ type: "clear" });
  return "";
}
