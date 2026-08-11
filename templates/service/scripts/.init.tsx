#!/usr/bin/env deno
import { join, resolve, normalize } from "@std/path";
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
    console.debug('Template Directory:', args.tpldir);
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
        // if (!args.flutter && entryPath.match(/lib$/i)) {
        //     return false;
        // }
    },
    onCopy(entryPath) {
        console.log(`Copying:`, entryPath);
        console.debug(`Flutter:`, args.flutter);
        
        // if (!args.flutter && entryPath.match(/lib[\\\/]{1}.*$/i)) {
        //     console.debug(`Is in lib ..`);
        //     return false;
        // }

        // if (!args.flutter && entryPath.match(/(.*\.dart)?$/i)) {
        //     console.debug(`Is dart ..`);
        //     return false;
        // }

        // if (!args.flutter && entryPath.match(/pubspec(\.yml|\.yaml\.tsx)?$/i)) {
        //     console.debug(`Is pubspec ..`);
        //     return false;
        // }

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

    // const denoWorkspace = {}; // TODO: new DenoWorkspace({ .. });
    // const denoWorkspaceDir = resolve("C:/Brainbow");
    // const denoWorkspaceConfigPath = resolve(denoWorkspaceDir, "deno.json");
    // const denoWorkspaceConfig0 = await Deno.readTextFile(denoWorkspaceConfigPath);
    // const denoWorkspaceConfig1 = denoWorkspaceConfig0
    //     // TODO: Move this to consts or statics or something ..
    //     // In the future, comments should be parsed to comments
    //     // and returned along with the JSON document.
    //     // .match(/(?:^|\s)\/\/(.*)|\/\*[\s\S]*?\*\//g, () => {
    //     //     console.warn(`TODO: Collect comments into denoWorkspace (look up) ..`);
    //     // })
    //     .replace(/(?:^|\s)\/\/(.*)|\/\*[\s\S]*?\*\//g, '');
    // const denoWorkspaceConfig = JSON.parse(denoWorkspaceConfig1);

    // lol .. lazy ..
    // const relativeProjectPath = OUT_DIR
    //     .replace(normalize(denoWorkspaceDir), '')
    //     .replaceAll(/\\/ig, '/')
    //     .replace(/^\//i, '');

    // if (!denoWorkspaceConfig.workspace.includes(relativeProjectPath)) {
    //     denoWorkspaceConfig.workspace = [...denoWorkspaceConfig.workspace, relativeProjectPath];
    //     console.log(`Added project to Deno workspace @ '${denoWorkspaceDir}'!`);
    // }

    // await Deno.writeTextFile(denoWorkspaceConfigPath, JSON.stringify(denoWorkspaceConfig, null, 2));
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
