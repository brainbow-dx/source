import { resolve } from "@std/path";
import { existsSync } from "@std/fs/exists";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.reset ??= false;

Deno.chdir(args.workdir);

const dataDir = await sh.homedir(".stooper/data", true);
const catalogDbPath = resolve(dataDir!, "catalog.db");
const catalogMigrationsPath = resolve("spec/store/catalog/.migrations/sqlite3/0000-init.sql");

await Deno.mkdir(dataDir!, { recursive: true });

if (await sh.which("sqlite3") == undefined) {
    // TODO: Offer to install sqlite?
    throw new Error(`Couldn't find SQLite installed. Have you bootstrapped your workspace yet?`);
}

if (args.reset == true && existsSync(catalogDbPath)) {
    console.log(`Got reset; nuking database '${catalogDbPath}`);
    await Deno.remove(catalogDbPath);
}

await $`sqlite3 ${catalogDbPath} < ${catalogMigrationsPath}`;