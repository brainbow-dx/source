#!/usr/bin/env deno
// deno-lint-ignore-file

import { join, resolve } from "@std/path";
import { exists } from "@std/fs";
import { serveDir } from "@std/http";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";
import type { Args } from "@ethos/dev/shell";

import { TerminalSurface } from "@escher/terminal";

//---
const args = sh.parse<Args>(Deno.args);

args.workdir ??= resolve(import.meta.dirname!, "../..");
args.target ??= "windows";

Deno.chdir(args.workdir);

const terminal = new TerminalSurface(Deno.stdout);

const entrypointPath = join(args.workdir, `./examples/avatar/index.html`);
const assetOutputPath = join(args.workdir, `./.output/pkg/web/public/avatar`);

// TODO: Replace this with a banner UI panel?
console.info(`Work Dir: ${Deno.cwd()}`);
console.info(`Entrypoint: ${Deno.cwd()}`);
// console.info(`Flutter Exe:`, await sh.which("flutter"));

await $`deno bundle ${entrypointPath} --outdir ${assetOutputPath} \
    --allow-import --allow-scripts --code-splitting --sourcemap \
    --platform browser`;

//---
Deno.serve({ port: 4956 }, async (request) => {
    const uriPath = new URL(request.url).pathname;
    const assetPath = join(assetOutputPath, uriPath);

    if (await exists(assetPath)) {
        console.debug(`Found Asset Path: ${assetPath}`);
        // TODO: Do something with the URL ..

        return serveDir(request, {
            fsRoot: assetOutputPath,
        });
    }

    console.debug(`Serving entrypoint ..`);
    return new Response("Hello!");
});