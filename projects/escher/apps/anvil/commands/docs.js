// Builds Escher's own mdbook documentation once, at startup (`onLoad`, run when this command is
// discovered rather than per invocation — see `AppState::spawn_command_onloads`), and mounts the
// built site behind `anvil://docs/...` via a real host action. This is a real exercise of the
// generic `op_host_action` mechanism for something beyond `quit`/`clear`: a real subprocess
// (`mdbook build`) and a new action type (`mountStaticDir`) the host side didn't know about when
// the mechanism itself was designed — the whole point of a generic `{ type, ...data }` message is
// that the host, not the mechanism, decides what a new `type` means.
//
// `import.meta.url` (this file's own real location), not `Deno.cwd()`, is what finds the docs
// source/output directories — Anvil's own cwd depends on where it was launched from (see
// `anvil_root()`'s own doc comment), but this script's position relative to the escher checkout
// (`apps/anvil/commands/docs.js`) is fixed regardless.

export const description = "Open Escher's built documentation book";

// Three levels up from this file's own directory (apps/anvil/commands/) reaches the escher
// project root: commands/ -> anvil/ -> apps/ -> escher/.
const DOCS_DIR = new URL("../../../docs", import.meta.url).pathname;
const OUTPUT_DIR = new URL("../../../.output/docs", import.meta.url).pathname;

// `postMessage(message)` is a thin, deliberately-familiar wrapper around the real host action op
// — same shape `commands/quit.js`/`commands/clear.js` already use; see either of those for the
// fuller explanation of why.
function postMessage(message) {
    if (typeof message === "string") message = { type: message };
    if (!message?.type) throw new TypeError("postMessage(message) needs a `type` field");
    globalThis.__ethosHostAction(message);
}

export const onLoad = async () => {
    let output;
    try {
        output = await new Deno.Command("mdbook", { args: ["build"], cwd: DOCS_DIR, stdout: "piped", stderr: "piped" }).output();
    } catch (error) {
        console.error(`Failed to run mdbook (is it installed and on PATH?): ${error}`);
        return;
    }

    if (!output.success) {
        console.error(`mdbook build failed: ${new TextDecoder().decode(output.stderr)}`);
        return;
    }

    postMessage({ type: "mountStaticDir", prefix: "docs", dir: OUTPUT_DIR });
};

export const run = async () => {
    postMessage({ type: "openUrl", url: "anvil://docs/" });
    return "";
};
