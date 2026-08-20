// deno-lint-ignore-file no-explicit-any
export type { Args } from "@std/cli";

import { build$ } from "@david/dax";
import type { CommandChild } from "@david/dax";

export * as dax from "@david/dax";
export * from "@david/which";

//--
import { join } from "@std/path";
import { exists } from "@std/fs";
import { parseArgs } from "@std/cli";
import type { Args } from "@std/cli";

export const $ = build$({
    commandBuilder: (builder) =>
        builder.stdout("inherit").stderr("inherit").stdin("inherit"),
});

/**
 * Mount a cli application.
 * @param args Set of string arguments to be parsed.
 * @returns Parsed arguments or nothing.
 */
export function parse<A extends Args>(args: string[] = [], setupFn?: (args: A) => void): any {
    $.setPrintCommand(true);

    const parsedArgs = parseArgs<A>(args);

    if (setupFn) {
        setupFn(parsedArgs as A);
    }

    return parsedArgs;
}

//---
/**
 * A dax command that hasn't been started yet, wrapped so it can be launched
 * later and killed on demand. `dax`'s `CommandChild` has no OS pid getter
 * (unlike the old dzx `Process`), so only start/stop is tracked here, not pid.
 */
export class ManagedProcess {
    #command: ReturnType<typeof $>;
    #child?: CommandChild;
    #running = false;

    constructor(
        command: ReturnType<typeof $>,
        public readonly options?: {
            timeout?: number,
            delay?: number,
            runImmediately?: boolean,
        },
    ) {
        this.#command = command.timeout(options?.timeout ?? 20000);

        if (this.options?.runImmediately) {
            this.run().then(() => {
                console.debug(`Auto-started process ..`);
            });
        }
    }

    public isRunning(): boolean {
        return this.#running;
    }

    public async run(): Promise<boolean> {
        if (this.#running) {
            return this.#running;
        }

        if (this.options?.delay) {
            await new Promise((resolve) => setTimeout(resolve, this.options!.delay));
        }

        this.#child = this.#command.spawn();
        this.#running = true;
        console.log(`Process started ..`);

        return this.#running;
    }

    public async kill(signal: Parameters<CommandChild["kill"]>[0] = "SIGTERM"): Promise<boolean> {
        if (this.#running && this.#child) {
            this.#child.kill(signal);
            this.#running = false;
        }

        return this.#running;
    }
}

//---
export function $$(strings: TemplateStringsArray, ...expressions: any[]): ManagedProcess {
    return new ManagedProcess($(strings, ...expressions));
};

export async function homedir(subDir: string, ensure?: boolean): Promise<string | undefined> {
    // TODO: What do we do on ios/android?
    const envHomeDir = Deno.build.os === "windows"
        ? Deno.env.get("USERPROFILE")
        : Deno.env.get("HOME");

    const homeDir = join(envHomeDir!, subDir);
    if (ensure && false == await exists(homeDir)) {
        await Deno.mkdir(homeDir, { recursive: true });
    }

    return homeDir;
}

export function banner(strings: TemplateStringsArray, ...values: any[]): string {
    const bannerBuilder = (bannerTemplate: string, chunk: string, i: number) => {
        bannerTemplate += chunk + (values[i] || '');
        return bannerTemplate;
    };

    const bannerTemplate = "";
    const bannerOutput = strings.reduce(bannerBuilder, bannerTemplate);

    // Find and sync with the first indent in the banner block.
    const lines = bannerOutput.split('\n');
    const indentMatch = lines.find(line => line.trim().length > 0)?.match(/^\s*/);
    const indent = indentMatch ? indentMatch[0].length : 0;
    return lines
        .filter(line => line.replaceAll(/\n/ig, ''))
        .map(line => line.substring(indent))
        .join('\n');
}
