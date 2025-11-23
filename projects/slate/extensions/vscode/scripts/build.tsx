#!/usr/bin/env deno
import { join, resolve } from "@std/path";
import { exists } from "@std/fs";


import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";
import type { Args } from "@brainbow/ethos/dev/shell";

import { build } from "@deno/dnt";

import { copyFiles } from "@brainbow/ethos/dev/fs";

import packageConfig from "../package.tpl.json" with { type: "json" };

// TODO: This.
// const denoEntrypoint = {};
// const nodeOutputDir = {};
// const result = await translate<Deno, Node>(denoEntrypoint, nodeOutputDir);

//---
const args = sh.parse<Args>(Deno.args);
// TODO: const tracer = new TracingSubscriber();

args.workdir ??= resolve(import.meta.dirname!, "..");
args.debug ??= undefined;
args.release ??= undefined;
args.verbose ??= false;
args.reset ??= true;
args.init ??= false;
args.generate ??= true;
args.proto ??= false; // TODO: Toggle when proto gen actually works!
args.bundle ??= true;
args.install ??= false;

const outputDir = join(args.workdir, "./.output");

const projectRoot = join(args.workdir, "../..");
const workspaceRoot = join(projectRoot, "../..");

const vscExtensionName = "Slate";
const vscExtensionKey = "slate-vscode";
const vscExtensionVersion = "0.0.0";

console.info(sh.banner`
    Workspace: ${projectRoot}
    Project: ${args.workdir}
    Rust: ${await sh.which("rustc") ?? "<not-found>"}
    Deno: ${await sh.which("deno") ?? "<not-found>"}
`);

try {
    Deno.chdir(args.workdir);

    if (args.init) {
        console.info(`Generating vscode extension ..`);

        // TODO: Prefer local templates here ..
        await $`npx -p yo -p generator-code -- \
            yo code .output/pkg \
                --extensionId=${vscExtensionKey} \
                --extensionDisplayName=${vscExtensionName} \
                --bundler=esm -t=ts --quick --force-install \
                --skip-cache --skip-install --skipOpen`;

        if (args.reset && await exists(join(outputDir, "pkg/src"))) {
            // await Deno.remove(join(outputDir, "./pkg/src/extension.ts"));
            await Deno.remove(join(outputDir, "pkg/src"), { recursive: true });
        }
    }

    if (args.proto) {
        await $`npx buf generate`;
    }

    if (args.proto) {
        await $`cbindgen
            --crate slate-vscode
            --output .output/pkg/include/slate_vscode.h`;
    }

    if (args.bundle) {
        // const browserBundle = await Deno.bundle({
        //     entrypoints: [
        //         "assets/editor/sketch.tsx",
        //         "assets/editor/markup.tsx",
        //     ],
        //     outputDir: ".output/pkg/web/assets/editor",
        //     codeSplitting: false,
        //     inlineImports: true,
        //     sourcemap: "inline",
        //     platform: "browser",
        //     packages: "bundle",
        //     format: "esm",
        //     minify: false,
        //     write: false,
        // });

        // if (browserBundle) {
        //     console.debug(`Browser Bundle:\n`, browserBundle);
        // }

        await $`deno bundle --platform browser --quiet \
            --allow-import --inline-imports --sourcemap --unstable \
            --outdir .output/pkg/web/assets/editor \
            assets/editor/sketch.tsx \
            assets/editor/markup.tsx`;
    }

    console.info(`Transpiling Deno-to-Node with dnt ..`);
    await build({
        entryPoints: [
            join(args.workdir, `src/extension.ts`),
        ],
        outDir: join(outputDir, "pkg"),
        esModule: true,
        scriptModule: false,
        skipSourceOutput: true,
        declaration: "separate",
        typeCheck: false,
        test: false,
        shims: {
            deno: true,
            blob: false,
            timers: true,
            prompts: false,
            webSocket: false,
            domException: false,
            // custom: [],
            // customDev: [],
        },
        package: {
            ...packageConfig,
            // name: vscExtensionKey,
            // TODO: We should figure this out wayyy up there ^^ ..
            main: `./esm/extension.js`,
        },
        compilerOptions: {
            lib: [
                "ES2020",
                "DOM",
                "Decorators",
                "ScriptHost",
                "WebWorker"
            ],
            target: "ES2020",
            sourceMap: true,
            inlineSources: true,
            experimentalDecorators: true,
            emitDecoratorMetadata: true,
        },
        async postBuild() {
            console.debug(`Post Build Args:`, arguments);

            Deno.chdir(join(outputDir, "./pkg"));
            console.debug(` .. in cwd:`, Deno.cwd());

            await Deno.copyFile(join(args.workdir, "LICENSE"), join(outputDir, "pkg/LICENSE"));
            await Deno.copyFile(join(args.workdir, "README.md"), join(outputDir, "pkg/README.md"));

            await Deno.mkdir(join(outputDir, "pkg/assets"), { recursive: true });

            // await Deno.copyFile(join(args.workdir, "assets/explorer/activity-icon.svg"), join(outputDir, "pkg/assets/explorer/actvity-icon.svg"));
            await copyFiles(join(args.workdir, "assets"), join(outputDir, "pkg/assets"), {
                walk: { exts: ["svg"] }
            });

            await copyFiles(join(projectRoot, "services/sketch/assets/fonts"), join(outputDir, "pkg/assets/fonts"), {
                walk: { exts: ["woff2"] }
            });

            if (args.install) {
                // TODO: Call the package script ..
                await $`npm run package`;
                await $`code --install-extension ${vscExtensionKey}-${vscExtensionVersion}.vsix`;
            }
        },
    });
} catch (exc) {
    console.warn(`Failed:`, exc);
}
