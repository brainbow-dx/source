import * as yaml from "jsr:@std/yaml";

export class ConfigFile {
    #$content?: string;

    constructor(content?: string) {
        this.#$content = content;
    }

    #$path?: string | URL;
    public get path() {
        return this.#$content;
    }

    //---
    public get content() {
        return this.#$content ?? "string";
    }
    public set content(value: string) {
        this.#$content = value;
    }
}

//---
const $$yaml = new ConfigFile(await import("./pubspec.yaml", { with: { type: "text" } }));
export default $$yaml;

// deno-lint-ignore require-await
export async function read(options?: { fetch: boolean }) {
    if (options?.fetch !== undefined) {
        console.warn(`TODO: Force-fetch new content from disk/wherever ..`);
    }

    return readSync();
}

export function readSync() {
    if ($$yaml.content) {
        yaml.parse($$yaml.content);
    }
}
