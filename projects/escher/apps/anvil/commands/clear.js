// A command script exports an async `run()` returning the reply text to show (or "" to show
// nothing, see below).
//
// Wipes the assistant's messages and tasks tables. Talks to sqld directly over its HTTP API
// (the same instance assistant.rs's Rust-side Persistence connects to) since this script has no
// access to the running process's own libsql connection.
//
// Returns the magic `CLEAR_SENTINEL` (main.rs) on success — a `/clear` that logged its own
// "js(clear)" / "Cleared all messages and tasks." lines right after clearing everything would
// leave exactly those two lines behind, the opposite of what "clear" means. `spawn_js_command`
// watches for this exact string and wipes the live, in-memory transcript too (not just `sqld`'s
// rows) — don't replace it with a friendlier string, that'll just silently break `/clear`. A real
// failure below still returns actual text, so genuine errors still show up.
//
// Needs `fetch()` to actually work inside the embedded JS engine — see `ethos-deno`'s `build.rs`
// (`RESIDUAL_LAZY_ESM_SOURCES`) for the real, separate bug that broke this until it was fixed:
// the embedded runtime's V8 snapshot didn't include `deno_fetch`'s lazy-loaded ESM sources, so
// any embedded command calling `fetch()` — this one included — failed outright with "cannot be
// lazy-loaded as it was not included in the binary."

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

  return "🧹";
}
