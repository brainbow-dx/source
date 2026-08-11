import { assert } from "@std/assert";
import { resolve } from "@std/path";
import { exists } from "@std/fs";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

const args = sh.parse(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, `..`);
args.reset ??= false;

Deno.chdir(args.workdir);

assert(await sh.which(`sqlite3`), `Couldn't find SQLite in the path.`);

const dataDir = await sh.homedir(`.escher/data`, true);
const schemaPath = resolve(`spec/store/notes/schema.sql`);
const dbPath = resolve(dataDir!, `notes.db`);

if (args.reset === true && await exists(dbPath)) {
    console.warn(`Got reset; nuking database @ '${dbPath}'`);
    await Deno.remove(dbPath);
}

await $`sqlite3 ${dbPath} < ${schemaPath}`;