// deno-lint-ignore-file
import { join, resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

//---
const args = sh.parse<Args>(Deno.args);
const production = Deno.args.includes("--production");
const watch = Deno.args.includes("--watch");

args.workdir ??= resolve(import.meta.dirname!, "..");

// TODO