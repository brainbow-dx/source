import { resolve } from "@std/path";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";

const args = sh.parse(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, `..`);
args.target ??= "windows";
args.reset ??= false;
args.generate ??= true;
args.migrate ??= true;
args.services ??= false;
args.shutdown ??= false;
args.clean ??= false;

Deno.chdir(args.workdir);

if (args.services) {
    await $`docker compose -f ./docker-compose.yaml up -d`;
}

if (args.services && args.shutdown) {
    await $`docker compose -f ./docker-compose.yaml down`;
}

if (args.clean) {
    await $`deno task clean`;
}
