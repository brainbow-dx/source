import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev";
import * as dev from "@brainbow/ethos/dev";

const args = dev.parseCli(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.generate ??= true;
args.migrate ??= true;
args.reset ??= false;

console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Flutter Exe:`, await dev.which("flutter"));

if (args.generate === true) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    await $`dart run build_runner build`;
}

if (args.migrate === true) {
    // TODO: Can we make --reset optional/inherited?
    args.reset ?
        await $`deno task migrate --reset` :
        await $`deno task migrate`;
}

// TODO: Should we use `--web-port 8082`?
await $`flutter run -d ${args.target} --hot`;
