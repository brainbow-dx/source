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

switch (args.target) {
    case "android":
        await $`flutter build apk`;
        break;
    case "ios":
        await $`flutter build ios`;
        break;
    case "tvos":
        await $`flutter build tvos`;
        break;
    default:
        await $`flutter build`;
        break;
}

if (args.clean) {
    await $`deno task clean`;
}
