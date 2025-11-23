#!/usr/bin/env deno
import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

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

Deno.chdir(args.workdir);

console.info(`Workspace: ${workspaceRoot}`);
console.info(`Project: ${Deno.cwd()}`);
console.info(`Dart Exe:`, $.blue(sh.whichSync("dart") ?? "<not-found>"));
console.info(`Flutter Exe:`, $.blue(sh.whichSync("flutter") ?? "<not-found>"));

if (args.generate === true) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    await $`dart run build_runner build`;
}

if (args.icons === true) {
    Deno.chdir(resolve(args.workdir, ".output/flutter"));
    // await $`dart run flutter_launcher_icons`;
}

// if (args.migrate === true) {
//     Deno.chdir(args.workdir);
//     // TODO: Can we make --reset optional/inherited?
//     args.reset ?
//         await $`deno task --cwd ../.. migrate --reset` :
//         await $`deno task --cwd ../.. migrate`;
// }

if (args.services === true) {
    Deno.chdir(args.workdir);
    // await $`docker compose -f ../../compose.yaml up -d`;
}

if (args.run === true) {
    Deno.chdir(resolve(args.workdir, ".output/flutter"));
    // TODO: Should we use `--web-port 8082`?
    args.target !== undefined
        ? await $`flutter run -d ${args.target} --hot`
        : await $`flutter run --hot`;
}

if (args.services === true && args.shutdown === true) {
    Deno.chdir(args.workdir);
    // await $`docker compose -f ../../compose.yaml down`;
}

if (args.clean === true) {
    Deno.chdir(args.workdir);
    await $`deno task clean`;
}
