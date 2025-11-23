#!/usr/bin/deno

import { join, resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import { parse, type Args } from "@brainbow/ethos/dev/shell";

// Does this retain comments now?
const args: Args = parse<Args>(Deno.args);

args.workdir ??= join(import.meta.dirname!, "..");
args.subcommand ??= args._[0]?.toString() ?? undefined;
args.dry ??= true; // TODO: Pass to `cargo run ..` below.
args.template ??= args._[1]?.toString() ?? undefined;
args.entrypoint ??= args._[2]?.toString() ?? undefined;

Deno.chdir(resolve(args.workdir));

if (!args.template) {
    throw new Error(`Missing Template argument ..`);
}

if (!args.entrypoint) {
    throw new Error(`Missing Entrypoint argument ..`);
}

console.log(`Work Dir:`, args.workdir);
console.log(`Entrypoint:`, args.entrypoint);

// TODO: Pass-through --dry from script args.
// TODO: We really should just pipe the tail args ..
await $`cargo run -- ${args.subcommand} ${args.template} ${args.entrypoint} --dry`;
