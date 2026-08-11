#!/usr/bin/env deno
// deno-lint-ignore-file no-explicit-any
import { join, resolve, normalize, dirname, basename } from "@std/path";
import * as path from "@std/path";
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
args.dry ??= false;
args.rust ??= false;
args.deno ??= false;
args.flutter ??= false;
args.name ??= undefined;
args.reset ??= false;
args.verbose ??= false;
args.inspect ??= false;
args.run ??= false;

//---
export const WORKSPACE_DIR: string = resolve("C:/Brainbow");

export const DEFAULT_PLATFORMS: string = "windows,mac,linux,ios,android,web";

export const OUT_DIR: string = resolve(Deno.cwd(), args._[0]?.toString() ?? ".");

if (args.verbose === true) {
    console.debug($.bold.magenta(`Args:`), args);
}

if (args.name == undefined) {
    args.name = basename(OUT_DIR);
    console.info(`No --name flag; using directory name '${args.name}' ..`);
}

//---
console.debug(`Generating ${args.name} @ '${OUT_DIR}'`);
console.debug(` .. from template @ '${args.tpldir}'`);

if (args.reset === true && await exists(OUT_DIR)) {
    console.warn($.yellow.bold(`Got reset! Removing '${OUT_DIR}' ..`));
    console.info(`Press <enter> to continue; ctrl+c to exit.`);

    const stdinBuf = new Uint8Array(100);
    if (Deno.stdin.readSync(stdinBuf) && args.verbose) {
        const userInput = new TextDecoder()
            .decode(stdinBuf)
            .replaceAll(/\n/ig, '');
        console.log(`Got input:`, userInput ?? '<enter>');
    }

    Deno.removeSync(OUT_DIR, { recursive: true });
}

// lol .. lazy ..
const relativeProjectPath = OUT_DIR
    .replace(normalize(WORKSPACE_DIR), '')
    .replaceAll(/\\/ig, '/')
    .replace(/^\//i, '');

if (!args.dry && !(await exists(OUT_DIR))) {
    await Deno.mkdir(OUT_DIR, { recursive: true });
}

//---
if (args.rust === true) {
    Deno.chdir(OUT_DIR);

    const cargoWorkspaceConfigPath = resolve(WORKSPACE_DIR, "deno.json");
    // const cargoWorkspaceConfigContents = (await Deno.readTextFile(cargoWorkspaceConfigPath))
    //     .replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
    // const cargoWorkspaceConfig = JSON.parse(cargoWorkspaceConfigContents);

    console.debug($.bold.magenta(`Rust`), cargoWorkspaceConfigPath);

    const match = [
        path.globToRegExp(`./Cargo.toml`),
        path.globToRegExp(`./scripts/*.rs`),
        path.globToRegExp(`./src/*.rs`),
    ];

    if (args.verbose && args.inspect) {
        console.debug(`Matching:`, match);
    }

    await copyFiles(args.tpldir, OUT_DIR, {
        dryRun: args.dry,
        walk: {
            exts: ["toml", "rs"],
        },
        onMakeDir(relPath: any) {
            if (args.verbose) {
                console.debug(`Creating directory @ '${relPath}'`);
            }
        },
        onCopy(content: any, relPath: string) {
            let isRustFile;

            for (const pattern of match) {
                if (pattern.test(relPath)) {
                    isRustFile = true;
                    break;
                }
            }

            if (!isRustFile) {
                if (args.verbose) {
                    console.debug(`Skipping '${relPath}'`);
                }
                return false;
            }

            // TODO: Process file for template tags.
            //  - Scan the document for comments.
            //  - Build a structured content editor for the document.
            //  - Iterate the document's structure, looking for template directives.

            if (args.verbose) {
                console.debug(`Copying '${relPath}'`);

                if (args.inspect) {
                    console.log('Parsed Path:', path.parse(relPath));
                    console.debug(content);
                }
            }
        },
    });

    if (!args.dry) {
        const cargoProjectConfigPath = resolve(OUT_DIR, "Cargo.toml");
        // deno-lint-ignore no-unused-vars
        const cargoProjectConfigContents = await Deno.readTextFile(cargoProjectConfigPath);
        // const cargoProjectConfig = JSON.parse(cargoProjectConfigContents);
        // 
        // TODO: Get the namespace from config?
        // cargoProjectConfig.name = `@brainbow/${args.name}`;
        // 
        // await Deno.writeTextFile(cargoProjectConfigPath, JSON.stringify(cargoProjectConfig, null, 2));
        // if (args.verbose) {
        //     console.info(`Updated project config:`, cargoProjectConfig);
        // }
    }
}

//---
if (args.deno === true) {
    Deno.chdir(OUT_DIR);

    const denoWorkspaceConfigPath = resolve(WORKSPACE_DIR, "deno.json");
    const denoWorkspaceConfigContents = (await Deno.readTextFile(denoWorkspaceConfigPath))
        .replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
    const denoWorkspaceConfig = JSON.parse(denoWorkspaceConfigContents);

    console.debug($.bold.magenta(`Deno`), denoWorkspaceConfigPath);

    const match = [
        // path.globToRegExp(`./pubspec.yaml`),
        path.globToRegExp(`./deno.json`),
        path.globToRegExp(`./scripts/*.tsx`),
        // path.globToRegExp(`./*{.jx|.jsx}`),
    ];

    if (args.verbose && args.inspect) {
        console.debug(`Matching:`, match);
    }

    await copyFiles(args.tpldir, OUT_DIR, {
        dryRun: args.dry,
        walk: {
            exts: ["json", "jsonc", "ts", "tsx", "js", "jsx"],
        },
        onMakeDir(relPath: any) {
            if (args.verbose) {
                console.debug(`Creating directory @ '${relPath}'`);
            }
        },
        onCopy(content: any, relPath: string) {
            let isDenoFile;

            for (const pattern of match) {
                if (pattern.test(relPath)) {
                    isDenoFile = true;
                    break;
                }
            }

            if (!isDenoFile) {
                if (args.verbose) {
                    console.debug(`Skipping '${relPath}'`);
                }
                return false;
            }

            if (args.verbose) {
                console.debug(`Copying '${relPath}'`);

                if (args.inspect) {
                    console.log('Parsed Path:', path.parse(relPath));
                    console.debug(content);
                }
            }
        },
    });

    if (!args.dry) {
        const denoProjectConfigPath = resolve(OUT_DIR, "deno.json");
        const denoProjectConfigContents = await Deno.readTextFile(denoProjectConfigPath);
        const denoProjectConfig = JSON.parse(denoProjectConfigContents);

        // TODO: Get the namespace from config?
        denoProjectConfig.name = `@brainbow/${args.name}`;

        await Deno.writeTextFile(denoProjectConfigPath, JSON.stringify(denoProjectConfig, null, 2));
        if (args.verbose) {
            console.info(`Updated project config:`, denoProjectConfig);
        }

        if (!denoWorkspaceConfig.workspace.includes(relativeProjectPath)) {
            denoWorkspaceConfig.workspace = [...denoWorkspaceConfig.workspace, relativeProjectPath];
            await Deno.writeTextFile(denoWorkspaceConfigPath, JSON.stringify(denoWorkspaceConfig, null, 2));
            console.info(`Added project to Deno workspace @ '${WORKSPACE_DIR}' ..`);
        }
    }
}

//---
if (args.flutter === true) {
    const projectName = flutter.toNameFrom(args.name);
    const project = new FlutterProject(projectName, {
        rootDir: OUT_DIR,
    });

    const flutterWorkspaceConfigPath = join(WORKSPACE_DIR, "pubspec.yaml");
    // const flutterWorkspaceConfigContents = await Deno.readTextFile(flutterWorkspaceConfigPath);
    // const flutterWorkspaceConfig = JSON.parse(flutterWorkspaceConfigContents);

    console.debug($.bold.magenta(`Flutter`), flutterWorkspaceConfigPath);

    const match = [
        path.globToRegExp(`./pubspec.yaml`),
        path.globToRegExp(`./lib/**/*.dart`),
    ];

    if (args.verbose && args.inspect) {
        console.debug(`Matching:`, match);
    }

    await copyFiles(args.tpldir, OUT_DIR, {
        dryRun: args.dry,
        walk: {
            exts: ["yaml", "dart"],
        },
        onMakeDir(relPath: any) {
            if (args.verbose) {
                console.debug(`Creating directory @ '${relPath}'`);
            }
        },
        onCopy(content: any, relPath: string) {
            let isFlutterFile;

            for (const pattern of match) {
                if (pattern.test(relPath)) {
                    isFlutterFile = true;
                    break;
                }
            }

            if (!isFlutterFile) {
                if (args.verbose) {
                    console.debug(`Skipping '${relPath}'`);
                }
                return false;
            }

            if (args.verbose) {
                console.debug(`Copying '${relPath}'`);

                if (args.inspect) {
                    console.log('Parsed Path:', path.parse(relPath));
                    console.debug(content);
                }
            }
        },
    });

    Deno.exit(1);

    if (!args.dry) {
        console.info(`Creating flutter project @ ${project.rootDir}`);

        await project.create({
            async onFinished() {
                // TODO
            }
        });

        await $`flutter pub get`;
    }
}

if (args.dry) {
    console.info($.bold(`Dry run. Exiting!`));
    Deno.exit(0);
}

//---
console.warn($.magenta.bold(`Fin. <3`));
// TODO: More information pls ..

//---
if (args.run) {
    await $`deno task --cwd ${OUT_DIR} dev`;
}