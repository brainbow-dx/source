// deno-lint-ignore-file no-unused-vars verbatim-module-syntax no-explicit-any
// TODO: Move this to the `@brainbow/ethos/flutter` package.
import { join, resolve } from "@std/path";
import { existsSync, walk } from "@std/fs";

import { $ } from "@ethos/dev/shell";

import { copyFiles } from "@ethos/dev/fs";

export function toNameFrom(sourceName: string): string {
    // We should try to convert from pascal case here too.
    return sourceName
        .replaceAll('-', '_')
        .toLowerCase();
}

export class FlutterProject {
    public readonly name: string;
    public readonly rootDir: string;
    public readonly outputDir: string = ".output";

    constructor(name: string, options?: Partial<FlutterProject>) {
        if (options?.rootDir == undefined) {
            throw new Error("missing required: rootDir");
        }

        Object.assign(this, options);

        this.name = name;
        this.rootDir = options.rootDir;
    }

    public get exists(): any {
        return existsSync(this.rootDir);
    }

    async create(options?: { platforms?: string[], onFinished?: () => void }): Promise<any> {
        const result = await $`flutter create --project-name ${this.name} -t package ${this.rootDir}`;

        // Ethos manages most of this with workspace tools + config.
        // TODO: Move tests to `../tests`?
        await Deno.remove(resolve(this.rootDir, `test`), { recursive: true });
        await Deno.remove(resolve(this.rootDir, `CHANGELOG.md`));
        await Deno.remove(resolve(this.rootDir, `LICENSE`));
        await Deno.remove(resolve(this.rootDir, `README.md`));

        // Can we get rid of this after the iml fix?
        // await Deno.remove(resolve(this.rootDir, `lib/${this.name}.dart`));
        // TODO: Edit the iml file to add the src, remove lib, etc.

        options?.onFinished?.call(this);

        return result;
    }

    async build(platform: string = "windows", targetDir?: string) {
        const prevDir = Deno.cwd();

        const outputDir = join(this.rootDir, this.outputDir);
        const outputFlutterDir = join(outputDir, "flutter");

        if (!existsSync(join(outputFlutterDir, platform))) {
            console.warn(`Output not yet generated! Creating @ '${outputFlutterDir}'`);
            // TODO: Deno.mkdirSync(outputFlutterDir, { recursive: true });
            await $`flutter create --project-name ${this.name}_app --platforms ${platform} ${outputFlutterDir}`;
        }

        // Change to the flutter dir (flutter doesn't have a "cwd" flag).
        Deno.chdir(outputFlutterDir);

        //--
        // Call out to Flutter to build the app.
        // await $`flutter build web`;
        const buildCommand = await $`flutter build ${platform}`;

        if (buildCommand.code > 0) {
            // TODO: Ensure the build command was successful before moving on ..
            console.debug(`Build Command Failed:`, buildCommand);
        }

        if (targetDir != undefined) {
            const outputTargetDir = resolve(outputDir, targetDir);
            await copyFiles(outputFlutterDir, outputTargetDir);
        }

        // Go back so the caller 
        Deno.chdir(prevDir);
    }
}
