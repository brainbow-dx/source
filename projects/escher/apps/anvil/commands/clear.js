// See greet.js for the slash-command convention.
//
// Wipes the assistant's messages and tasks tables. Talks to sqld directly over its HTTP API
// (the same instance assistant.rs's Rust-side Persistence connects to) since this script has no
// access to the running process's own libsql connection.

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

  return "Cleared all messages and tasks.";
}
