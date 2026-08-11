#!/usr/bin/env deno
import { globToRegExp, join, parse, ParsedPath, resolve } from "@std/path";
import { debounce } from "@std/async/debounce";

import { $, ManagedProcess } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";
import type { Args } from "@ethos/dev/shell";

import { Workspace } from "@ethos/workspace";
import { serveDev } from "@ethos/dev";

// # IMPORTANT:
// For the moment, if this stops, it won't try to restart and you'll likely
// have an orphan process. Use `lsof -i :3000` and `kill -9 [pid]`.
// ## TODO:
//  - Listen for Exit Code 127 and also restart.
//  - Attempt to restart when the service fails for any reason.

//---
const args = sh.parse<Args>(Deno.args);

args.cwd ??= resolve(import.meta.dirname!, "..");
args.hostname ??= Deno.env.get("DEV_SERVER_HOSTNAME") ?? "localhost";
args.port ??= Deno.env.get("DEV_SERVER_PORT") ?? (9000).toString();
args.target ??= "windows";
args.server ??= true;
args.generate ??= true;
args.build ??= false;
args.run ??= true;
args.clean ??= false;

Deno.chdir(args.cwd);

const composeCommand = $`docker compose up --build --profiles dev --watch auth`;
const composeProcess = new ManagedProcess(composeCommand, {
    // delay: 1000,
    timeout: 10000,
});

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
    const devCommand = $`cargo run -p escher --example serve --features dev -- --cwd ${args.cwd} --address 0.0.0.0:3615`;
    const devProcess = new ManagedProcess(devCommand, {
        delay: 100,
        timeout: 5000,
    });
    
    await devProcess.run();
    
    const workspace = new Workspace(args.cwd);
    
    const outputDirPattern = globToRegExp("**/.output/**");
    
    await serveDev(workspace, devProcess, {
        hostname: args.hostname,
        port: parseInt(args.port),
        outputDir: args.outdir,
        async onFsEvent(event) {
            try {
                let shouldBuild = false;
                let shouldRestart = false;
                
                for (const eventPath of event.paths) {
                    const path = parse(eventPath);
                    
                    if (outputDirPattern.test(path.dir)) {
                        continue;
                    }
                    else if (/\.(html|htmx|md|mdx|svg|css|json?)$/.test(path.ext)) {
                        shouldBuild = true;
                    }
                    else if (/\.(rs|ts|tsx?)$/.test(path.ext)) {
                        shouldBuild = true;
                        shouldRestart = true;
                    }
                    else if (["Cargo.toml", "deno.json"].includes(path.base)) {
                        shouldBuild = true;
                        shouldRestart = true;
                    }
                }
                
                if (devProcess.isRunning() && shouldRestart) {
                    // TODO: Try to run shutdown operations first?
                    await devProcess.kill();
                }
                
                if (args.build && shouldBuild) {
                    await $`deno task build`;
                }
                
                if (args.server && !composeProcess.isRunning() && shouldRestart) {
                    await composeProcess.run();
                }
                
                if (args.run && !devProcess.isRunning() && shouldRestart) {
                    await devProcess.run();
                }
            } catch (error: unknown) {
                // TODO: Optionally alert the user?
                console.error(`Dev build failed!`, error);
                alert(`Dev build failed: ${error}`);
            }
        },
    })
}
