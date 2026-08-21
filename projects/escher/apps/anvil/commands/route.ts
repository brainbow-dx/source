// Routes text the shell fallback rejected outright (see `AppState::spawn_shell_command`'s
// `ShellOutcome::Rejected` handling in `main.rs`) through a chain of increasingly capable agents.
// It's not a replacement for real command dispatch, just a catch for "that wasn't gibberish, it just
// wasn't spelled as a slash command." Not discovered as a slash command itself
// (`discover_js_commands` only scans for `.js`, this is `.ts`. Same reason `commands/shape.tsx`
// isn't auto-discovered either).
//
// Rust doesn't know anything about routing at all, not even what a "command" is. `main.rs`
// doesn't build a tool list and hand it in. This script takes the raw rejected text as its one
// argument and does
// everything itself: discovers what's routable by scanning `commands/` (this file's own
// directory; `Deno.cwd()` is `apps/anvil`, set by `AppState::spawn_shell_command` via
// `run_reject_router`, the same convention Rust's own `discover_js_commands` uses) plus the two
// commands that aren't backed by a script file at all (`task`/`scene`, hardcoded in `main.rs`'s
// `builtin_commands`, mirrored here since there's no file to discover them from).
//
// Three tiers, each strictly more expensive/capable than the last, tried in order until one
// produces something:
//   1. `tier1LocalToolRouter`: a fast local Ollama tool-call gut check against Anvil's own real
//      command list, tested against a real local Ollama instance, not hypothetical. If a tool
//      clearly matches, that's the answer. If the model replies with plain text instead of
//      calling a tool, that reply becomes the "updated message" tier 2 receives instead of the
//      original. It's a first-pass rephrase/clarification, not a second opinion on whether a tool
//      matches.
//   2/3. `escalateTo`: pluggable, not yet backed by anything real, built out ahead of time so a
//      real tier 2/3 is a config change later, not new code. Each is a plain HTTP POST to a
//      configured URL, `{"message": "..."}` in, the exact same `{"replace": string | null}` shape
//      this whole script already emits back out. So wiring a real tier 2/3 (a bigger local
//      model, `eden` once it has a working inference backend, a hosted API behind a small proxy)
//      later needs zero changes here or on the Rust side, just standing up a service at
//      `ANVIL_TIER2_URL`/`ANVIL_TIER3_URL`. Unset (the default) skips straight through, so
//      behavior stays tier 1 (or nothing) unchanged until one is configured.
//
// Prints exactly one JSON line: `{"replace": "/command args", "reply": null}` if a real command
// matched, or `{"replace": null, "reply": "..."}` (tier 1's own conversational reply: a typo
// correction, an answer to a greeting/question, whatever) when nothing resolved to a command
// but the model still had something worth telling the user, or `{"replace": null, "reply": null}`
// if there's truly nothing (including if every configured tier is unreachable, times out, or
// returns something unparseable). A missing suggestion is always a silent, safe outcome for the
// caller; neither field is ever required for the shell-fallback path to keep working.

const OLLAMA_URL = Deno.env.get("ANVIL_OLLAMA_URL") ?? "http://localhost:11434/api/chat";
// Whatever's configured, this should be a model that actually reports `"tools"` in its
// `/api/tags` capabilities. `gpt-oss:latest` does, and reliably makes correct tool calls for
// this exact use case.
const MODEL = Deno.env.get("ANVIL_OLLAMA_MODEL") ?? "gpt-oss:latest";
// Unset by default. See this file's own top doc comment for the escalation contract each of
// these is expected to speak.
const TIER2_URL = Deno.env.get("ANVIL_TIER2_URL");
const TIER3_URL = Deno.env.get("ANVIL_TIER3_URL");
const ESCALATION_TIMEOUT_MS = 20_000;

interface Tool {
  name: string;
  description: string;
  /// Empty means the command takes no arguments.
  argsHint: string;
}

// `main.rs`'s `builtin_commands()` hardcodes these three as native Rust behavior with `script:
// None`, including `shape`, even though `commands/shape.tsx` is its real implementation file:
// it runs through its own bespoke `spawn_shape_command` path, not the generic file-discovery one,
// so it belongs here, not in the directory scan below. Descriptions mirror `main.rs` verbatim so
// the router's idea of what these do doesn't drift from the real thing.
const NATIVE_TOOLS: Tool[] = [
  { name: "task", description: "Add a task to the overlay", argsHint: "<label>" },
  { name: "scene", description: "Open a Bevy scene with a webview", argsHint: "<url>" },
  { name: "shape", description: "Render tonight's demo shape (terminal/web/Unity)", argsHint: "" },
];

async function discoverTools(): Promise<Tool[]> {
  const tools = [...NATIVE_TOOLS];
  try {
    // Only `.js`, matching `discover_js_commands` on the Rust side exactly. `.ts`/`.tsx` files
    // (this script itself, `shape.tsx`) are deliberately not auto-discovered there either.
    for await (const entry of Deno.readDir("commands")) {
      if (!entry.isFile || !entry.name.endsWith(".js")) continue;
      const name = entry.name.slice(0, -".js".length);
      tools.push({
        name,
        description: `Run ${name} (commands/${name}.js)`,
        argsHint: name === "clear" ? "" : "<args>",
      });
    }
  } catch {
    // Missing/unreadable `commands/` just means no discovered tools beyond the native ones.
    // Same "optional, not load-bearing" stance `discover_js_commands` takes on the Rust side.
  }
  return tools;
}

function toOllamaTools(tools: Tool[]) {
  return tools.map((tool) => ({
    type: "function",
    function: {
      name: tool.name,
      description: tool.description,
      parameters: tool.argsHint
        ? {
          type: "object",
          properties: { args: { type: "string", description: `Arguments matching the hint: ${tool.argsHint}` } },
          required: ["args"],
        }
        : { type: "object", properties: {} },
    },
  }));
}

interface Tier1Result {
  /// A tool clearly matched. That's the final answer, nothing else runs.
  command: string | null;
  /// Set only when the model replied with plain text instead of calling a tool. It's a first-pass
  /// rephrase of `input` for tier 2 to consider instead of the raw original. `null` (not just
  /// falling back to `input`) when the model gave nothing usable either way, so a caller can tell
  /// "rephrased to nothing new" apart from "here's a real rephrase."
  updatedMessage: string | null;
}

async function tier1LocalToolRouter(input: string, tools: Tool[]): Promise<Tier1Result> {
  const response = await fetch(OLLAMA_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    // 4s was tried first and found wrong via a real live test, not assumed: a genuine tool-call
    // response from `gpt-oss:latest` measured `total_duration` of ~8.3s (~2.4s just loading the
    // model, per Ollama's own reported `load_duration`), timing out and reading indistinguishably
    // from "no tool matched" every time. 20s comfortably covers a cold-load response.
    signal: AbortSignal.timeout(20_000),
    body: JSON.stringify({
      model: MODEL,
      messages: [
        {
          role: "system",
          content:
            "You are the fallback assistant for a terminal app called Anvil. The user just typed text that " +
            "wasn't a recognized shell command or slash command. If one of the available tools clearly " +
            "matches their intent, call it, filling in any arguments as well-formed values (e.g. a bare " +
            "domain becomes a real https:// URL). Otherwise, don't call a tool — reply directly instead: " +
            "if their text looks like a typo of a real shell command or one of your own tools, say what " +
            "you think they meant; if it's a greeting, question, or general remark, just respond to it " +
            "naturally and briefly, like a helpful assistant would. Keep any reply short, a sentence or two.",
        },
        { role: "user", content: input },
      ],
      tools: toOllamaTools(tools),
      stream: false,
    }),
  });

  if (!response.ok) return { command: null, updatedMessage: null };

  const json = await response.json();
  const call = json.message?.tool_calls?.[0];
  if (call?.function?.name) {
    const args = call.function.arguments?.args ?? "";
    return { command: args ? `/${call.function.name} ${args}` : `/${call.function.name}`, updatedMessage: null };
  }

  const content = typeof json.message?.content === "string" ? json.message.content.trim() : "";
  return { command: null, updatedMessage: content || null };
}

/// A pluggable tier 2/3 escalation step. See this file's own top doc comment for the contract.
/// `url` unset means this tier doesn't exist yet, so this is a no-op;
/// unreachable/timed-out/unparseable all collapse to the same "this tier had nothing" outcome as
/// a deliberate no-op, same stance every tier in this chain already takes.
async function escalateTo(url: string | undefined, message: string): Promise<string | null> {
  if (!url) return null;

  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      signal: AbortSignal.timeout(ESCALATION_TIMEOUT_MS),
      body: JSON.stringify({ message }),
    });
    if (!response.ok) return null;

    const json = await response.json();
    return typeof json.replace === "string" ? json.replace : null;
  } catch {
    return null;
  }
}

try {
  const input = Deno.args[0] ?? "";
  const tools = await discoverTools();

  const tier1 = await tier1LocalToolRouter(input, tools);
  // Tier 2/3 both consider whatever tier 1's own rephrase settled on, falling back to the raw
  // input untouched when tier 1 had nothing to add. Either way, it's "the original message itself or
  // an updated message," never both at once.
  const messageForEscalation = tier1.updatedMessage ?? input;

  const replace = tier1.command ?? (await escalateTo(TIER2_URL, messageForEscalation)) ?? (await escalateTo(TIER3_URL, messageForEscalation));
  // Only surfaced once nothing resolved to a real command to run. Tier 1's own reply is the one
  // thing left worth showing the user instead of silently doing nothing.
  const reply = replace ? null : tier1.updatedMessage;
  console.log(JSON.stringify({ replace: replace ?? null, reply }));
} catch {
  console.log(JSON.stringify({ replace: null, reply: null }));
}
