import { resolve } from "@std/path";

import { $, parseCli, which } from "@brainbow/ethos/dev";

const args = parseCli(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.reset ??= false;

Deno.chdir(args.workdir);

const dataDir = resolve(".stooper/data");
const catalogDbPath = resolve(dataDir, "catalog.db");
const catalogMigrationPath = resolve("schema/catalog/.migrations/0000-init.sql");

await Deno.mkdir(dataDir, { recursive: true });

if (await which("sqlite3") == undefined) {
    // TODO: Offer to install sqlite?
    throw new Error(`Couldn't find SQLite installed. Have you bootstrapped your workspace yet?`);
}

if (args.reset == true) {
    console.log(`Got reset; nuking database '${catalogDbPath}`);
    await Deno.remove(catalogDbPath);
}

await $`sqlite3 ${catalogDbPath} < ${catalogMigrationPath}`;