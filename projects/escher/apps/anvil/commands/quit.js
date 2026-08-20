// `postMessage(message)` is a thin, deliberately-familiar wrapper around the real host action op
// (`ethos_deno::host_actions`/`spec/.agents/proposals/anvil-command-host-api.md`) — same public
// shape a script author already knows from `Worker`/`BroadcastChannel` messaging, even though the
// actual mechanism underneath is a direct host call, not real cross-realm message passing (that
// wouldn't structurally fit "script calls into its own host"; see the proposal doc for why). The
// whole message survives the round trip, not just a squeezed-out type name — a `{ type, ...data }`
// object is passed straight through to the host as real, structured data.
function postMessage(message) {
    if (typeof message === "string") message = { type: message };
    if (!message?.type) throw new TypeError("postMessage(message) needs a `type` field");
    globalThis.__ethosHostAction(message);
}

export const run = async () => {
    postMessage({ type: "quit" });
    return "Goodbye.";
};
