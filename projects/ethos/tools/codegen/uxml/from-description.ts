// A pure `ScaffoldDescription → {uxml, uss}` transform — no authored content of its own. Takes a
// `ScaffoldDescription` JSON string as `args` (see `apps/cli/src/main.rs`'s `RunCommand` doc
// comment: "Passed as a single string to the script's exported `run` function") and returns
// `{uxml, uss}` as JSON.
//
// Supersedes `shape-demo.ts` (removed 2026-08-16), which conflated two different jobs in one
// file: authoring a demo shape *and* compiling it to UXML/USS. Per the repo owner's direction,
// those are now split at the process boundary — "build the Scaffold" is Escher's job
// (`apps/anvil/commands/shape.tsx`, JSX-authored, run via plain `deno run` since `ethos-deno` has
// no JSX/real-TS support — see `mod.ts`'s header comment), "compile a Scaffold description to a
// target format" is this repo's job. `apps/anvil/src/shape.rs` runs the two in sequence and pipes
// the first's stdout into this script's `args`.
//
// Plain JS syntax despite the `.ts` extension — see `mod.ts`'s header comment for why (no
// TS-stripping pass in `ethos-deno` yet).

import { scaffoldDescriptionToUxml } from "./mod.ts";

export function run(args) {
    // Plain `console.log` throughout, same reasoning as `apps/anvil/commands/shape.tsx` — it
    // already streams live into Anvil's output UI, and the caller reads only the *last* stdout
    // line as the real payload (matching `ethos-cli run-command`'s own "console.log output, then
    // the return value, last" convention), so any number of progress lines before it are free.
    console.log("Parsing scaffold description...");
    const description = JSON.parse(args);

    console.log("Compiling to UXML/USS...");
    const { uxml, uss } = scaffoldDescriptionToUxml(description, "Shape.uss");

    console.log("Done.");
    return JSON.stringify({ uxml, uss });
}
