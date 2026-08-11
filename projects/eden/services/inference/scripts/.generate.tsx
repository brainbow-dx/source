#!/usr/bin/env deno
import { join, resolve, normalize, dirname, basename } from "@std/path";
import { exists, existsSync, walk, WalkOptions } from "@std/fs";

import { $ } from "@ethos/dev/shell";
import * as sh from "@ethos/dev/shell";

import * as flutter from "@ethos/flutter";
import { FlutterProject } from "@ethos/flutter";

import { copyFiles } from "@ethos/dev/fs";

interface Args extends sh.Args {
    rust?: boolean,
    deno?: boolean,
    flutter?: boolean,
    verbose?: boolean,
    run?: boolean,
}

const args = sh.parse<Args>(Deno.args);

args.tpldir ??= resolve(import.meta.dirname!, "..");
args.platforms ??= "windows,macos,linux,ios,android,web";
args.rust ??= true;
args.deno ??= true;
args.flutter ??= false;
args.name ??= undefined;
args.reset ??= false;
args.verbose ??= false;
args.run ??= false;

//---
export const DEFAULT_PLATFORMS = "windows,mac,linux,ios,android,web";

export const OUT_DIR = resolve(Deno.cwd(), args._[0]?.toString() ?? ".");

if (args.verbose === true) {
    console.debug(`Args:`, args);
}

if (args.name == undefined) {
    args.name = basename(OUT_DIR);
    console.info(`No --name flag; using directory name '${args.name}' ..`);
}

//---
console.debug(`Generating ${args.name} @ '${OUT_DIR}'`);

if (args.reset === true && existsSync(OUT_DIR)) {
    console.warn($.yellow.bold(`Got reset! Removing '${OUT_DIR}' ..`));
    console.info(`Press <enter> to continue; ctrl+c to exit.`);

    const buf = new Uint8Array(100);
    if (Deno.stdin.readSync(buf)) {
        const text = new TextDecoder().decode(buf);
        console.log(`Got input:`, text);
    }

    Deno.removeSync(OUT_DIR, { recursive: true });
}

await copyFiles(args.tpldir, OUT_DIR, {
    onMakeDir(fromPath, toPath) {
        if (!args.flutter && toPath.match(/lib$/i)) {
            return false;
        }

        if (args.verbose) {
            console.debug(`Creating directory @ '${fromPath}'`);
        }
    },
    onCopy(fromPath, toPath) {
        if (!args.flutter && toPath.match(/lib[\\\/]{1}.*$/i)) {
            return false;
        }

        if (!args.flutter && toPath.match(/pubspec(\.yaml|\.yaml\.tsx)?$/i)) {
            return false;

        }

        if (args.verbose) {
            console.log(`Copying file '${fromPath}'`);
        }
    },
});

//---
if (args.deno === true) {
    Deno.chdir(OUT_DIR);

    const denoProjectConfigPath = resolve("deno.json");
    const denoProjectConfigContents = await Deno.readTextFile(denoProjectConfigPath);
    const denoProjectConfig = JSON.parse(denoProjectConfigContents);

    // TODO: Get the namespace from config?
    denoProjectConfig.name = `@brainbow/${args.name}`;

    await Deno.writeTextFile(denoProjectConfigPath, JSON.stringify(denoProjectConfig, null, 2));

    const denoWorkspaceDir = resolve("C:/Brainbow");
    const denoWorkspaceConfigPath = resolve(denoWorkspaceDir, "deno.json");
    const denoWorkspaceConfigContents = await Deno
        .readTextFileSync(denoWorkspaceConfigPath)
        .replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
    const denoWorkspaceConfig = JSON.parse(denoWorkspaceConfigContents);

    // lol .. lazy ..
    const relativeProjectPath = OUT_DIR
        .replace(normalize(denoWorkspaceDir), '')
        .replaceAll(/\\/ig, '/')
        .replace(/^\//i, '');

    if (!denoWorkspaceConfig.workspace.includes(relativeProjectPath)) {
        denoWorkspaceConfig.workspace = [...denoWorkspaceConfig.workspace, relativeProjectPath];
        console.info(`Added project to Deno workspace @ '${denoWorkspaceDir}' ..`);
    }

    await Deno.writeTextFile(denoWorkspaceConfigPath, JSON.stringify(denoWorkspaceConfig, null, 2));
}

//---
if (args.flutter === true) {
    const projectName = flutter.toNameFrom(args.name);
    const project = new FlutterProject(projectName, {
        rootDir: OUT_DIR,
    });

    console.info(`Creating flutter project @ ${project.rootDir}`);

    let createResult = await project.create({
        // Note: These are set in the app in the `.output/flutter` dir.
        platforms: args.platforms,
        async onFinished() {
            // TODO
        }
    });

    // TODO: Move this to a method on the project.
    await $`flutter pub get`;

    console.info(`Building flutter project @ ${project.rootDir}`);

    await project.build("windows");
}

//---
console.warn($.magenta.bold(`Fin. <3`));
// TODO: More information pls ..

//---
if (args.run) {
    await $`deno task --cwd ${OUT_DIR} dev`;
}