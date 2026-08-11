#!/usr/bin/env deno
import { globToRegExp, parse, resolve } from "@std/path";

import { $, ManagedProcess } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

import { Workspace } from "@ethos/workspace";
import { serveDev } from "@ethos/dev";
import { join } from "node:path";

export interface Args extends sh.Args {
    
}

const args = sh.parse<sh.Args>(Deno.args);

// TODO: Move most of these to parse defaults (as second arg) ..
args.workdir ??= resolve(import.meta.dirname!, "..");
args.target ??= "windows"; // TODO: Get current env.
args.build ??= true;
args.port ??= 8081;
args.debug ??= undefined;
args.release ??= undefined;
args.reset ??= true;
args.generate ??= true;
args.icons ??= true;
args.migrate ??= true;
args.services ??= true;
args.shutdown ??= true;
args.run ??= true;
args.clean ??= false;

const workspaceRoot = resolve(args.workdir, "../..");

Deno.chdir(args.workdir);

console.info(`Workspace: ${workspaceRoot}`);
console.info(`Project: ${Deno.cwd()}`);
console.info(`Dart Exe:`, $.blue(sh.whichSync("dart") ?? "<not-found>"));
console.info(`Flutter Exe:`, $.blue(sh.whichSync("flutter") ?? "<not-found>"));

if (args.generate === true) {
    // TODO: Audit:
    //  - Is this necessary?
    //  - Can we make it faster?
    // TODO: Probably better to move this to a build.ts, anyway.
    await $`dart run build_runner build`;
}

if (args.icons === true) {
    await $`dart run flutter_launcher_icons`;
}

if (args.migrate === true) {
    // TODO: Can we make --reset optional/inherited?
    args.reset
        ? await $`deno task --cwd ../.. migrate --reset`
        : await $`deno task --cwd ../.. migrate`;
}

if (args.services === true) {
    await $`docker compose -f ../../compose.yaml up -d`;
}

function devCommand(target: string) {
    switch (target) {
        case "chrome":
        case "edge":
        case "firefox":
            return $`flutter run -d ${args.target} --hot --web-port ${args.port}`;
        case "windows":
        case "macos":
        case "linux":
            return $`flutter run -d ${args.target} --hot`;
        default:
            return $`flutter run --hot`;
    }
}

if (args.run === true) {
    const workspace = new Workspace(args.cwd);
    const outputDirRel = ".output";
    const outputDirPattern = globToRegExp(`**/${outputDirRel}/**`);
    
    // TODO: Probably want to move this to docker compose with a custom build
    //.  layer for the flutter web container.
    const buildProcess = new ManagedProcess($`flutter build web`, {
        timeout: 5000,
        runImmediately: true,
    });
    
    const devProcess = new ManagedProcess(devCommand(args.target), {
        delay: 100,
        timeout: 5000,
        runImmediately: true,
    });
    
    await serveDev(workspace, devProcess, {
        hostname: args.hostname,
        port: parseInt(args.port),
        serveDir: join(outputDirRel, "pkg/web/public"),
        outputDir: outputDirRel,
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
                
                if (devProcess?.isRunning() && shouldRestart) {
                    // TODO: Try to run shutdown operations first?
                    await devProcess?.kill();
                }
                
                if (args.build && shouldBuild) {
                    await $`deno task build`;
                    await buildProcess.run();
                }
                
                if (args.run && !devProcess?.isRunning() && shouldRestart) {
                    await devProcess?.run();
                }
            } catch (error: unknown) {
                // TODO: Optionally alert the user?
                console.error(`Dev build failed!`, error);
                alert(`Dev build failed: ${error}`);
            }
        },
    })
}

if (args.services === true && args.shutdown === true) {
    await $`docker compose -f ../../compose.yaml down`;
}

if (args.clean === true) {
    await $`deno task clean`;
}
