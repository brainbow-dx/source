import { resolve } from "@std/path";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

const args = sh.parse(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows";

Deno.chdir(args.workdir);

console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Flutter Exe:`, await sh.which("flutter"));

// await $`nu ./scripts/bundle.nu`;

await $`cargo build`;
