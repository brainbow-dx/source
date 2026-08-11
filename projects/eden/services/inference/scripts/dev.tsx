#!/usr/bin/deno
import { resolve } from "@std/path";

import { $ } from "@ethos/dev/shell";
import { parse, type Args } from "@ethos/dev/shell";

const args = parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.clean ??= false;

Deno.chdir(args.workdir);

console.log(`Work Dir:`, args.workdir);

await $`maturin develop --uv`;

await $`../../.venv/Scripts/python ./src/main.py`;

if (args.clean) {
    // await $`deno clean`;
    await $`cargo clean`;
    await $`flutter clean`;

    // TODO: Can we do this more simply?
    Deno.removeSync(`./src/__pycache__`, { recursive: true });
}