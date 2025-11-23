#!/usr/bin/env deno
import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

//---
const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.debug ??= undefined;
args.release ??= undefined;
args.verbose ??= false;
args.reset ??= true;
args.generate ??= false;
args.icons ??= true;

// TODO