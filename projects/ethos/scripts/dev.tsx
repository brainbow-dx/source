#!/usr/bin/env deno

import { join, resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import { parse } from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

const args = parse<Args>(Deno.args);

args.workdir ??= join(import.meta.dirname!, "..");
args.dry ??= true; // TODO: Use for `cargo command run ..` below.

const subcommand = args._[0]?.toString() ?? undefined;

if (!subcommand) {
    throw new Error(`Missing Sub Command ..`);
}

Deno.chdir(resolve(args.workdir));

const template = args._[1]?.toString() ?? undefined;
const entrypoint = args._[2]?.toString() ?? undefined;

if (!template) {
    throw new Error(`Missing Template arg ..`);
}

if (!entrypoint) {
    throw new Error(`Missing Entrypoint arg ..`);
}

console.log(`Work Dir:`, args.workdir);
console.log(`Entrypoint:`, entrypoint);

// TODO: Pass-through --dry from script args.
// TODO: We really should just pipe the tail args ..
await $`cargo run -- ${args.subcommand} ${template} ${entrypoint} --dry`;