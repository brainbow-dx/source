#!/usr/bin/deno

import { resolve } from "@std/path";

import { $ } from "#src/shell.ts";
import { parse } from "#src/shell.ts";

const args = parse(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.generate ??= true;

console.info(`Work Dir: ${args.workdir}`);
Deno.chdir(args.workdir);

if (args.generate === true) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    await $`echo 'TODO: Generate Command'`;
}

if (args.target === "windows") {
    // TODO
}
