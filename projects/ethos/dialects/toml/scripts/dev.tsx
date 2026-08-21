#!/usr/bin/env deno
import { resolve } from "@std/path";
import { blue, red } from "@std/fmt/colors";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";
import type { Args } from "@ethos/dev/shell";

const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.debug ??= undefined;
args.release ??= undefined;
args.reset ??= true;
args.generate ??= true;
args.icons ??= true;
args.migrate ??= true;
args.services ??= true;
args.shutdown ??= true;
args.run ??= true;
args.clean ??= false;

const workspaceRoot = resolve(args.workdir, "../..");

console.info(red(`
    Workspace: 
    Project: 
    Rust: 
    Deno: 
    //#tmpl.include()
`));

Deno.chdir(args.workdir);

console.info(`Workspace: ${workspaceRoot}`);
console.info(`Project: ${Deno.cwd()}`);
//#tmpl.include(args.rust)
console.info(`Rust`, blue(await sh.which("rustc") ?? "<not-found>"));
//#tmpl.include(args.deno)
console.info(`Deno`, blue(await sh.which("deno") ?? "<not-found>"));
//#tmpl.include(args.flutter)
console.info(`
    ${blue("Dart")}: ${await sh.which("dart") ?? "<not-found>"} \n
    ${blue("Flutter")}: ${await sh.which("flutter") ?? "<not-found>"}
`);

//#tmpl.include(args.deno)
if (args.generate === true) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    // await $`dart run build_runner build`;
}

if (args.clean === true) {
    Deno.chdir(args.workdir);
    //#tmpl.include(args.rust)
    await $`cargo clean`;
    //#tmpl.include(args.deno)
    await $`deno task clean`;
    //#tmpl.include(args.flutter)
    await $`flutter clean`;
}
