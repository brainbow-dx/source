import { join, resolve } from "@std/path";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

import { bundle } from "@ethos/dev/bundle";

// import denoConfig from "../deno.json" with { type: "json" };

//---
const args = sh.parse(Deno.args);

args.cwd ??= resolve(import.meta.dirname!, "..");
args.debug ??= undefined;
args.release ??= undefined;
args.reset ??= true;
args.generate ??= true;
args.icons ??= true;
args.clean ??= false;
args.dry ??= false;

Deno.chdir(args.cwd);

const workspaceRoot = resolve(args.cwd, "../..");

console.info(sh.banner`
  Workspace: ${workspaceRoot}
    Project: ${args.cwd}
       Rust: ${await sh.which("rustc") ?? "<not-found>"}
       Deno: ${await sh.which("deno") ?? "<not-found>"}
`);

//--
await $`cargo build --lib`;

// TODO: Build browser wasm ..
// await $`wasm-pack build -p stooper-mobile --lib`;
await $`cargo build --lib --target wasm32-unknown-unknown`;

//--
await $`deno bundle \
  --platform browser --unstable --quiet \
  --allow-import --inline-imports --sourcemap \
  --outdir .output/pkg/web \
  assets/web/index.html \
  assets/web/draw.html \
  assets/web/404.html`;
