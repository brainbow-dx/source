#!/usr/bin/env deno
import { join, resolve } from "@std/path";
import { debounce } from "@std/async/debounce";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";
import type { Args } from "@ethos/dev/shell";

//---
const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "..");
args.outdir ??= resolve(args.workdir!, ".output");
args.debug ??= undefined;
args.release ??= undefined;
args.verbose ??= false;
args.reset ??= true;
args.generate ??= true;
args.migrate ??= true;
args.services ??= true;
args.run ??= true;
args.watch ??= false;
args.reload ??= false;

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

export async function runBuild(event: Deno.FsEvent = { kind: "other", paths: [] }) {
    console.debug("[%s] %s", event.kind, event.paths);

    Deno.chdir(args.workdir);

    // deno-lint-ignore prefer-const
    let shouldCodegen = false;
    let shouldBundle = false;

    for (const path of event.paths) {
        if (path.includes(join(args.workdir, "assets"))) {
            shouldBundle = true;
        }
        else if (path.includes(join(args.workdir, "src"))) {
            shouldBundle = true;
        }
    }

    if (shouldCodegen && shouldBundle) {
        await $`deno task build --generate`;
    } else {
        await $`deno task build`;
    }

    if (args.run) {
        const packageDir = resolve(args.outdir, "./pkg");
        await $`code --extensionDevelopmentPath=${packageDir}`;
    }
}

try {
    if (args.watch) {
        await watchDir(args.workdir, {
            outputDir: args.outdir,
            onWatchEvent: runBuild,
        })
    }
} catch (exc) {
    console.error(`Failed:`, exc);
}