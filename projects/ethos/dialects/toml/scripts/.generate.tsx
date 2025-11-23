#!/usr/bin/env deno
import { join, resolve, normalize, basename, globToRegExp, parse as parsePath } from "@std/path";
import { exists, walk, WalkOptions } from "@std/fs";

import { $ } from "@brainbow/ethos/dev/shell";
import * as sh from "@brainbow/ethos/dev/shell";

import * as flutter from "@brainbow/ethos-flutter";
import { FlutterProject } from "@brainbow/ethos-flutter";

import { copyFiles } from "@brainbow/ethos/dev/fs";

// TODO: Move this to ethos/os.
function isValidPath(relPath: string, include?: RegExp[], exclude?: RegExp[]) {
    if (include && include.length > 0) {
        for (const pattern of include) {
            if (pattern.test(relPath)) {
                return true;
            }
        }
    }

    if (exclude && exclude.length > 0) {
        for (const pattern of exclude) {
            if (pattern.test(relPath)) {
                return false;
            }
        }
    }
};

//---
interface Args extends sh.Args {
    rust?: boolean,
    deno?: boolean,
    flutter?: boolean,
    verbose?: boolean,
    inspect?: boolean,
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
export const DEFAULT_PLATFORMS = "windows,mac,linux,ios,android,web";

export const ETHOS_WORKSPACE_DIR = resolve("C:/Brainbow");

export const OUT_DIR = resolve(Deno.cwd(), args._[0]?.toString() ?? ".");

if (args.verbose && args.inspect) {
    console.debug($.brightBlue(`Args:`), args);
}

if (!args.name) {
    args.name = basename(OUT_DIR);
    console.info(`Missing --name flag; using directory name '${args.name}' ..`);
}

//---
console.debug(`Generating ${args.name} @ '${OUT_DIR}'`);
console.debug(` .. from ${args.tpldir}`);
console.debug(` .. to ${OUT_DIR}`);

if (args.reset === true && await exists(OUT_DIR)) {
    console.warn($.brightYellow(`Got reset:`), `Nuke and replace '${OUT_DIR}'?`);

    // Confirm the destruction of the previous directory ..
    const encoder = new TextEncoder();
    const confirm = encoder.encode($.dim.italic(`Press <enter> to continue or <ctrl+c> to exit ..`));
    await Deno.stdout.write(confirm);

    const stdinBuffer = new Uint8Array(1);
    if (await Deno.stdin.read(stdinBuffer) && args.verbose) {
        // Re-iterate intent to destroy.
        const decoder = new TextDecoder();
        const _input = decoder.decode(stdinBuffer);
        console.log($.magenta.italic(`Roger, go to boom.`));
    }

    // TODO: Flush much?? Gross ..
    Deno.stdin.close();

    Deno.removeSync(OUT_DIR, { recursive: true });
}

// lol lazy ..
const relativeProjectPath = OUT_DIR
    .replace(normalize(ETHOS_WORKSPACE_DIR), '')
    .replaceAll(/\\/ig, '/')
    .replace(/^\//i, '');

if (!args.dry && !(await exists(OUT_DIR))) {
    await Deno.mkdir(OUT_DIR, { recursive: true });
}

//---
if (args.rust === true) {
    Deno.chdir(OUT_DIR);

    const cargoWorkspaceConfigPath = resolve(ETHOS_WORKSPACE_DIR, "deno.json");
    // const cargoWorkspaceConfigContents = (await Deno.readTextFile(cargoWorkspaceConfigPath))
    //     .replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
    // const cargoWorkspaceConfig = JSON.parse(cargoWorkspaceConfigContents);

    console.debug($.brightBlue(`Rust`), cargoWorkspaceConfigPath);

    const exts = ["toml", "rs"];
    const include = [
        globToRegExp("./Cargo.toml"),
        globToRegExp("./scripts/**/*.rs"),
        globToRegExp("./src/**/*.rs"),
    ];
    const exclude = [
        globToRegExp("./.output/**/*"),
    ];

    console.debug(`Moving project files ..`);

    await copyFiles(args.tpldir, OUT_DIR, {
        dryRun: args.dry,
        walk: {
            exts,
            // Bugged??
            // match: include,
            // skip: exclude,
        },
        onCopy(_content, relPath) {
            if (!isValidPath(relPath, include, exclude)) {
                console.debug($.dim(`x skip ${relPath}`));
                return false;
            }

            // TODO: Process file for template tags.
            //  - Scan the document for comments.
            //  - Build a structured content editor for the document.
            //  - Iterate the document's structure, looking for template directives.

            if (args.verbose) {
                console.debug($.dim(`>`), `copy ${relPath}`);

                if (args.inspect) {
                    // console.log('Parsed:', parsePath(relPath));
                    // console.debug(_content);
                }
            }
        },
        onMakeDir(relPath) {
            if (args.verbose) {
                console.debug($.dim(`  /`), `make ${relPath}`);
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

    const denoWorkspaceConfigPath = resolve(ETHOS_WORKSPACE_DIR, "deno.json");
    const denoWorkspaceConfigContents = (await Deno.readTextFile(denoWorkspaceConfigPath))
        .replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
    const denoWorkspaceConfig = JSON.parse(denoWorkspaceConfigContents);

    console.debug($.brightBlue(`Deno`), denoWorkspaceConfigPath);

    const exts = ["json", "jsonc", "ts", "tsx", "js", "jsx"];
    const include = [
        globToRegExp("./deno.{json,jsonc}"),
        globToRegExp("./scripts/**/*.{ts,tsx,js,jsx}"),
        globToRegExp("./src/**/*.{ts,tsx,js,jsx}"),
    ];
    const exclude = [
        globToRegExp("./.output/**/*"),
    ];

    console.debug(`Moving project files ..`);

    await copyFiles(args.tpldir, OUT_DIR, {
        dryRun: args.dry,
        walk: {
            exts,
        },
        onCopy(_content, relPath) {
            if (!isValidPath(relPath, include, exclude)) {
                console.debug($.dim(`x skip ${relPath}`));
                return false;
            }

            if (args.verbose) {
                console.debug($.dim(`>`), `copy ${relPath}`);

                if (args.inspect) {
                    // TODO: Draw in an inline Slate tui block?
                    // console.log(`Kablow:`, parsePath(relPath));
                    // console.debug(_content);
                }
            }
        },
        onMakeDir(relPath) {
            if (args.verbose) {
                console.debug($.dim(`  /`), `make ${relPath}`);
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
            console.info(`Updated new deno config @ ${denoProjectConfigPath}:`);

            if (args.inspect) {
                console.debug(denoProjectConfig);
            }
        }

        if (!denoWorkspaceConfig.workspace.includes(relativeProjectPath)) {
            denoWorkspaceConfig.workspace = [...denoWorkspaceConfig.workspace, relativeProjectPath];
            await Deno.writeTextFile(denoWorkspaceConfigPath, JSON.stringify(denoWorkspaceConfig, null, 2));
            console.info(`Added project to Deno workspace @ '${ETHOS_WORKSPACE_DIR}' ..`);
        }
    }
}

//---
if (args.flutter === true) {
    const projectName = flutter.toNameFrom(args.name);
    const project = new FlutterProject(projectName, {
        rootDir: OUT_DIR,
    });

    const workspacePubspecPath = join(ETHOS_WORKSPACE_DIR, "pubspec.yaml");
    // const workspacePubspecContents = await Deno.readTextFile(workspacePubspecPath);
    // const workspacePubspec = JSON.parse(workspacePubspecContents);

    console.debug($.brightBlue(`Flutter`), workspacePubspecPath);

    const tmplPubspecPath = globToRegExp("./pubspec.{yml,yaml}");

    const exts = ["yaml", "dart"];
    const include = [
        tmplPubspecPath,
        globToRegExp("./lib/**/*.dart"),
        globToRegExp("./scripts/**/*.dart"),
    ];
    const exclude = [
        globToRegExp("./.output/**/*"),
    ];

    console.debug(`Moving project files ..`);

    await copyFiles(args.tpldir, OUT_DIR, {
        dryRun: args.dry,
        walk: {
            exts,
        },
        onCopy(content, relPath) {
            if (!isValidPath(relPath, include, exclude)) {
                console.debug($.dim(`Skipping ${relPath}`));
                return false;
            }

            if (args.verbose) {
                console.debug($.dim(`>`), `copy ${relPath}`);

                if (args.inspect) {
                    console.log('Parsed Path:', parsePath(relPath));
                    console.debug(content);
                }
            }
        },
        onMakeDir(relPath) {
            if (args.verbose) {
                console.debug($.dim(`  /`), `make ${relPath}`);
            }
        },
    });

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