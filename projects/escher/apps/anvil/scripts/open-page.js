// Polls `url` until something actually answers, before Anvil navigates a browser tab to it —
// per the user directly: it's this step's own job to make sure there's a real page to show
// before asking the browser to show it, not Anvil firing the navigation the instant it asks a
// server to start and hoping the timing works out. Confirmed live as a real bug otherwise:
// `/relay-console` started its static-file server (`serve-relay-console.ts`) and pushed the
// navigation in the same tick — `Deno.serve`'s own startup (binding the port, loading the
// module) has real, measurable latency, so the webview's request usually arrived before
// anything was listening, landing on a blank page with no automatic retry.
//
// Runs through the embedded JS engine (`ethos-deno`), same as `commands/clear.js` — real
// `fetch()` support confirmed working there this session, after fixing a real gap in that
// engine's own V8 snapshot.
//
// Returns "" once `url` responds (any response at all, even an error page, means something is
// actually listening — the page itself can show whatever error it wants from here), or a real
// error message if `timeoutMs` elapses first.
//
// Each individual `fetch` gets its own short deadline (`perAttemptMs`), separate from the
// overall `timeoutMs` this function bounds itself to — confirmed live as a real, silent-hang
// bug otherwise: a `fetch` against a port nothing is actually listening on doesn't necessarily
// reject quickly (observed live taking well past this function's own outer 15s timeout with no
// error surfacing at all — Anvil just never opened the tab, and never said why), so without a
// per-attempt bound, one bad connection attempt could block every retry this function is
// otherwise built to make.
export async function run(argsJson) {
  const { url, timeoutMs = 15000, perAttemptMs = 2000 } = JSON.parse(argsJson);
  const deadline = Date.now() + timeoutMs;
  let lastError = "unknown error";

  while (Date.now() < deadline) {
    try {
      await fetch(url, { method: "HEAD", signal: AbortSignal.timeout(perAttemptMs) });
      return "";
    } catch (error) {
      lastError = error.message;
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }

  return `Timed out after ${timeoutMs}ms waiting for ${url} to respond (${lastError})`;
}
