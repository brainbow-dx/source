#!/usr/bin/env deno
import { resolve } from "@std/path";
import { exists } from "@std/fs";

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
args.clean ??= false;
args.dry ??= false;

Deno.chdir(args.workdir);

const workspaceRoot = resolve(args.workdir, "../..");

console.info(sh.banner`
    Workspace: ${workspaceRoot}
    Project: ${Deno.cwd()}
    Rust: ${$.blue(await sh.which("rustc") ?? "<not-found>")}
    Deno: ${$.blue(await sh.which("deno") ?? "<not-found>")}
`);

if (args.reset && await exists(`.output/pkg/web`)) {
    await Deno.remove(`.output/pkg/web`, { recursive: true });
}

if (args.generate) {
    // await $`deno bundle`;
}

if (!args.dry) {
    // await $`cargo build -p escher-web --lib --target wasm32-unknown-unknown`;
    await $`wasm-pack build --dev --no-pack --target web \
        --out-name escher --out-dir .output/pkg/web`;
    
    await $`deno bundle --platform browser --quiet \
        --allow-import --inline-imports --sourcemap --unstable \
        --outdir .output/pkg/web \
        assets/download/index.html \
        assets/404.html \
        assets/draw.html \
        assets/index.html`;
}

if (args.clean) {
    await $`cargo clean`;
    await $`deno task clean`;
}
