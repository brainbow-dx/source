// deno-lint-ignore-file no-explicit-any
export type { Args } from "@std/cli";

export { $ } from "dzx";
export * as dzx from "dzx";

export * from "dzx/runtime/process";

export * from "@david/which";

//--
import { join } from "@std/path";
import { exists } from "@std/fs";
import { parseArgs } from "@std/cli";
import type { Args } from "@std/cli";

import { $ } from "dzx";
import type { Process } from "dzx/runtime/process";

/**
 * Mount a cli application.
 * @param args Set of string arguments to be parsed.
 * @returns Parsed arguments or nothing.
 */
export function parse<A extends Args>(args: string[] = [], setupFn?: (args: A) => void): any {
    $.verbose = 2;
    $.shell = "bash";
    $.stdout = "inherit";
    $.stderr = "inherit";
    $.stdin = "inherit";

    const parsedArgs = parseArgs<A>(args);

    if (setupFn) {
        setupFn(parsedArgs as A);
    }

    return parsedArgs;
}

//---
/**
 * TODO: Deprecate this?
 */
export class ManagedProcess {
    private running: boolean = false;
    
    constructor(
        private readonly process: Process,
        public readonly options?: {
            timeout?: number,
            delay?: number,
            runImmediately?: boolean,
        },
    ) {
        process.timeout(options?.timeout ?? 20000);
        process.delay(options?.delay ?? 3000);
        
        if (this.options?.runImmediately) {
            this.run().then(() => {
                console.debug(`Auto-started process ..`);
            });
        }
    }
    
    public timeout(duration: number): Process {
        return this.process.timeout(duration);
    }
    
    public delay(duration: number): Process {
        return this.process.delay(duration);
    }
    
    public isRunning(): boolean {
        return this.running;
    }
    
    public async run(): Promise<any> {
        return await new Promise((resolve, reject) => {
            try {
                // Accessing the `pid` getter internally calls the original
                // `process` getter (see original code), which creates and
                // starts the Deno.Process, initializing this.#proc.
                // We intentionally ignore the return value here.
                if (!this.running && this.process?.pid) {
                    this.running = true; // TODO: Wait + check?
                    console.log(`Process started with PID '${this.process.pid}' ..`);
                }
            } catch (error) {
                return reject(error);
            }
            
            return resolve(this.running);
        })
    }
    
    public async kill(signal: Deno.Signal = "SIGTERM"): Promise<any> {
        return await new Promise((resolve, reject) => {
            try {
                if (this.running && this.process?.pid) {
                    this.process.kill(signal);
                    this.running = false; // TODO: Wait + check?
                }
            } catch (error) {
                return reject(error);
            }
            
            return resolve(this.running);
        })
    }
}

//---
export function $$(strings: TemplateStringsArray, ...expressions: any[]): ManagedProcess {
    // for (let i = 0; i < strings.length; i++) {
    //     if (expressions[i] == undefined) {
    //         strings[i] += strings[i + 1];
    //     }
    // }
    
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
    
    const bannerTemplate = ""; // TODO: GEt this from disk/env/config?
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