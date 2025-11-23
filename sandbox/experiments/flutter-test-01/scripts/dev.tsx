#!/usr/bin/deno

import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import { parse, type Args } from "@brainbow/ethos/dev/shell";

const args = parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.generate ??= true;

Deno.chdir(args.workdir);

if (true === args.generate) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    await $`echo 'TODO: Generate Command'`;
}

Deno.chdir(resolve(args.workdir, ".output/flutter"));

await $`flutter run`;
