import { join, resolve } from "@std/path";
import { debounce } from "@std/async/debounce";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";
import { ManagedProcess } from "@ethos/dev/shell";

const args = sh.parse<sh.Args>(Deno.args);

args.cwd ??= resolve(import.meta.dirname!, "..");
args.workspace ??= resolve(import.meta.dirname!, "../../..");
args.generate ??= true;
args.build ??= true;
args.run ??= true;
args.clean ??= false;
args.watch ??= false;

Deno.chdir(args.cwd);

if (args.generate === true) {
    // await $`deno bundle`;
}

if (args.build) {
    await $`deno task build --reset --generate`;
}

if (args.run) {
    // const desktopCommand = $`deno desktop main.ts`;
    await $`cargo run --example serve --features dev -- --address 127.0.0.1:3615 --workspace ${args.workspace}`;
    /*
    const runCommand = $`cargo run --example serve --features dev -- --address 127.0.0.1:3615 --workspace ${args.workspace}`;
    
    const runProcess = new ManagedProcess(runCommand, {
        timeout: 10000,
        delay: 1000,
    });
    
    await runProcess.run();
    
    if (args.watch) {
        await watchDir(args.cwd, {
            outputDir: args.outdir,
            async onWatchEvent(event) {
                let shouldBuild = false;
                let shouldRestart = false;
                
                for (const path of event.paths) {
                    // Assume things in the asset dir should be re-built on change.
                    if (path.includes(join(args.cwd, "assets"))) {
                        shouldBuild = true;
                    }
                    // .. and things in src should both re-build and recompile.
                    else if (path.includes(join(args.cwd, "src"))) {
                        shouldBuild = true;
                        shouldRestart = true;
                    }
                }
                
                if (runProcess.isRunning() && shouldRestart) {
                    await runProcess.kill();
                }
                
                if (args.build && shouldBuild) {
                    await $`deno task build`;
                }
                
                if (args.run && !runProcess.isRunning() && shouldRestart) {
                    await runProcess.run();
                }
            },
        })
    }
    */
}

if (args.clean === true) {
    await $`cargo clean`;
    await $`deno task clean`;
}

export interface WatchEvent extends Deno.FsEvent {
    //..
}

export type WatchFn = (event: Deno.FsEvent) => void;

export interface WatchOptions {
    outputDir?: string;
    runOnStartup?: boolean | (() => boolean | undefined | void);
    onWatchEvent?(event: Deno.FsEvent): void;
}

export async function watchDir(path: string, options?: WatchOptions) {
    const outputDir = options?.outputDir ?? join(path, "./.output");

    if (options?.onWatchEvent) {
        const recursive = true;
        const onWatchEventFn = debounce(options.onWatchEvent, 1000);

        if (options?.runOnStartup === true) {
            onWatchEventFn({ kind: "other", paths: [] });
        }

        console.debug(`Watching for changes in '${path}' ..`);

        for await (const event of Deno.watchFs(path, { recursive })) {
            if (["access"].includes(event.kind)) {
                continue;
            }

            let foundChanges = 0;
            let shouldUpdate = false;

            for (const eventPath of event.paths) {
                if (outputDir && eventPath.startsWith(outputDir)) {
                    continue;
                }
                foundChanges++;
                shouldUpdate = true;
            }

            if (shouldUpdate && options.onWatchEvent instanceof Function) {
                onWatchEventFn(event);
            }
        }
    }
}
