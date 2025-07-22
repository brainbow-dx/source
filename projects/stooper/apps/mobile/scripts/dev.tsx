#!/usr/bin/env deno
import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.reset ??= false;
args.generate ??= true;
args.migrate ??= true;
args.services ??= false;
args.shutdown ??= false;
args.clean ??= false;

console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Dart Exe:`, $.blue(sh.whichSync("dart") ?? "<not-found>"));
console.info(`Flutter Exe:`, $.blue(sh.whichSync("flutter") ?? "<not-found>"));

if (args.generate) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    await $`dart run build_runner build`;
}

if (args.migrate) {
    // TODO: Can we make --reset optional/inherited?
    args.reset ?
        await $`deno task --cwd ../.. migrate --reset` :
        await $`deno task --cwd ../.. migrate`;
}

if (args.services) {
    await $`docker compose -f ../../compose.yaml up -d`;
}

// TODO: Should we use `--web-port 8082`?
await $`flutter run -d ${args.target} --hot`;

if (args.services && args.shutdown) {
    await $`docker compose -f ../../compose.yaml down`;
}

if (args.clean) {
    await $`deno task clean`;
}
