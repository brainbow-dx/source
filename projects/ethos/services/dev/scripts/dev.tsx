#!/usr/bin/deno
// deno-lint-ignore-file
import { globToRegExp, join, parse, ParsedPath, resolve } from "@std/path";
import { debounce } from "@std/async/debounce";

import { $, ManagedProcess } from "#src/shell.ts";
import * as sh from "#src/shell.ts";
import type { Args } from "#src/shell.ts";

// Note: For the moment, if this stops, it won't try to restart and you'll
// likely have an orphan process. Use `lsof -i :3000` and `kill -9 [pid]`.

// TODO: Listen for Exit Code 127 and also restart.
// TODO: Attempt to restart when the service fails for any reason.

import type { ReactNode } from "react";

import { serveDev } from "#src/server.ts";

// import { TerminalSurface } from "@escher/terminal";
import { Workspace } from "@ethos/sdk/workspace";

//---
interface DevArgs extends Args {
    cwd?: string,
    workspace?: string,
}

const args = sh.parse<DevArgs>(Deno.args);

args.cwd ??= resolve(import.meta.dirname!, "..");
args.server ??= true;
args.generate ??= true;
args.build ??= true;
args.run ??= true;
args.clean ??= false;

Deno.chdir(args.cwd);

const composeCommand = $`docker compose up --build --profiles dev --watch`;
const composeProcess = new ManagedProcess(composeCommand, {
    // delay: 1000,
    timeout: 10000,
});

// const terminal = new TerminalSurface(Deno.stdout);

//---
if (args.generate === true) {
    // await $`deno bundle`;
}

if (args.build) {
    await $`deno task build --reset --generate`;
}

if (args.server) {
    console.debug(`Running Docker Compose`);
    await composeProcess.run();
}

if (args.run) {
    const workspace = new Workspace(args.cwd);
    
    const devProcess = new ManagedProcess($`cargo run --example serve --features dev`, {
        delay: 100,
        timeout: 5000,
    });
    
    const ignorePatterns = globToRegExp("**/.output/**");
    
    // await devProcess.run();
    
    await serveDev(workspace, devProcess, {
        outputDir: args.outdir,
        onFsEvent(_) {
            try {
                // TODO
            } catch (error: unknown) {
                console.error(`Dev build failed!`, error);
            }
        },
    })
}
