import { resolve } from "@std/path";
import { existsSync } from "@std/fs";
import { assert } from "jsr:@std/assert";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";

const args = sh.parse(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, `..`);
args.reset ??= false;

Deno.chdir(args.workdir);

assert(await sh.which(`sqlite3`), `Couldn't find SQLite in the path.`);

const dataDir = await sh.homedir(`.slate/data`, true);
await Deno.mkdir(dataDir!, { recursive: true });

const schemaPath = resolve(`spec/store/notes/schema.sql`);
const dbPath = resolve(dataDir!, `notes.db`);

if (args.reset === true && existsSync(dbPath)) {
    console.warn(`Got reset; nuking database @ '${dbPath}'`);
    await Deno.remove(dbPath);
}

await $`sqlite3 ${dbPath} < ${schemaPath}`;