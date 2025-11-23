#!/usr/bin/env deno
import { join, resolve, normalize } from "@std/path";
import { exists, existsSync, walk, WalkOptions } from "@std/fs";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";

import * as flutter from "@brainbow/ethos-flutter";
import { FlutterProject } from "@brainbow/ethos-flutter";

import { copyFiles } from "@brainbow/ethos/dev/fs";

interface Args extends sh.Args {
    rust?: boolean,
    deno?: boolean,
    flutter?: boolean,
    verbose?: boolean,
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

//---
export const DEFAULT_PLATFORMS = "windows,mac,linux,ios,android,web";

export const OUT_DIR = resolve(args._[0]?.toString() ?? ".");

if (args.verbose === true) {
    console.debug('Output Directory:', OUT_DIR.replace(/\\/ig, '/'));
}

if (args.name == undefined) {
    throw new Error(`Please gimme a name!?`);
}

if (args.reset === true && existsSync(OUT_DIR)) {
    console.warn($.yellow.bold(`Got reset! Removing existing files ..`));
    await Deno.remove(OUT_DIR, { recursive: true });
}

await copyFiles(args.tpldir, OUT_DIR, {
    onMakeDir(entryPath) {
        if (!args.flutter && entryPath.match(/lib$/i)) {
            return false;
        }
    },
    onCopy(entryPath) {
        // TODO: Normalize path for simpler regex patterns ..

        if (!args.flutter && entryPath?.match(/lib[\\\/]+.*$/i)) {
            return false;
        }

        if (!args.flutter && entryPath?.match(/pubspec\.?[a-z]{3,4}$/i)) {
            return false;

        }

        if (args.verbose) {
            console.log(`Copying file '${entryPath}'`);
        }
    },
});

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
    const denoWorkspaceConfigContents = await Deno.readTextFile(denoWorkspaceConfigPath);
    const denoWorkspaceConfig = JSON.parse(denoWorkspaceConfigContents);

    // lol .. lazy ..
    const relativeProjectPath = OUT_DIR
        .replace(normalize(denoWorkspaceDir), '')
        .replaceAll(/\\/ig, '/')
        .replace(/^\//i, '');

    if (!denoWorkspaceConfig.workspace.includes(relativeProjectPath)) {
        denoWorkspaceConfig.workspace = [...denoWorkspaceConfig.workspace, relativeProjectPath];
        console.log(`Added project to Deno workspace @ '${denoWorkspaceDir}' ..`);
    }

    await Deno.writeTextFile(denoWorkspaceConfigPath, JSON.stringify(denoWorkspaceConfig, null, 2));
}

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
            // await copyFiles(args.tpldir, project.rootDir);
        }
    });

    console.info(`Building flutter project @ ${project.rootDir}`);

    await project.build("windows");
}

Deno.chdir(OUT_DIR);

console.warn($.magenta.bold(`Fin. <3`));
