import { join, resolve } from "@std/path";
import { red } from "@std/fmt/colors";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

import * as terminal from "@escher/sdk/terminal";

// import denoConfig from "../deno.json" with { type: "json" };

//---
const args = sh.parse(Deno.args);

args.cwd ??= resolve(import.meta.dirname!, "..");
args.workdir ??= resolve(args.cwd, "../..");
args.debug ??= undefined;
args.release ??= undefined;
args.reset ??= true;
args.generate ??= true;
args.icons ??= true;
args.clean ??= false;
args.dry ??= false;

if (args.cwd != undefined) {
    Deno.chdir(args.cwd);
}

args.cwd = undefined;

console.info(sh.banner`
    Project: ${args.cwd ?? red("<unknown>")}
  Workspace: ${args.workdir ?? red("<unknown>")}
    Version: ${await sh.which("rustc") ?? "<not-found>"}
       Rust: ${await sh.which("rustc") ?? "<not-found>"}
       Deno: ${await sh.which("deno") ?? "<not-found>"}
`);

//--
await $`cargo build -p escher --lib`;

// TODO: Build browser wasm ..
// await $`wasm-pack build -p escher` --lib`;
await $`cargo build -p escher --target wasm32-unknown-unknown --lib`;

terminal.draw(surface => {
    console.debug(`Drawing surface:`, surface);
    //..
    return (
        <h1>Dang??!</h1>
    )
});