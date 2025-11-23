import { join, resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";

import { bundle } from "@brainbow/ethos/dev/bundle";

// import denoConfig from "../deno.json" with { type: "json" };

//---
const args = sh.parse(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";
args.vscode ??= true;

Deno.chdir(args.workdir);

console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Flutter Exe:`, await sh.which("flutter"));

//--
await $`cargo build -p slate-sketch --lib`;

// TODO: Build browser wasm ..
// await $`wasm-pack build -p slate-sketch --lib`;
await $`cargo build -p slate-sketch --target wasm32-unknown-unknown --lib`;

//--
// const browserBundle = await bundle([
//   "assets/web/layouts/default.html",
//   "assets/web/layouts/draw.html",
// ], {
//   //..
// });

// if (browserBundle) {
//   console.debug(`Browser Bundle:\n`, browserBundle);
// }

console.info(`Bundling for 'browser' targets ..`);
await $`deno bundle --platform browser --quiet \
  --allow-import --inline-imports --sourcemap --unstable \
  --outdir .output/pkg/web/public \
  assets/web/index.html \
  assets/web/draw.html`;
