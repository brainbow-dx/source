// A classic (non-module) extension script, run eagerly during extension setup rather than
// waiting on an explicit `import`. Captures a direct reference to this extension's own op
// function right now, at extension-init time, rather than a lazy `Deno.core.ops.op_host_action`
// lookup evaluated later -- a later lookup breaks two different ways: `deno_runtime`'s own
// bootstrap (`99_main.js`'s `removeImportedOps`) deletes any op not on its own hardcoded
// allowlist from `Deno.core.ops` by the time a command script actually runs, and separately
// replaces the entire `Deno` namespace object wholesale (`ObjectDefineProperty(globalThis,
// "Deno", ...)`), which drops `Deno.core` off the public namespace entirely. Grabbing the
// function reference itself, right now, sidesteps both. `commands/quit.js`/`commands/clear.js`
// then just call `globalThis.__ethosHostAction(message)`, the same way `console.log`/`fetch` are
// already ordinary globals to them.
const op_host_action = Deno.core.ops.op_host_action;
globalThis.__ethosHostAction = (message) => op_host_action(message);
