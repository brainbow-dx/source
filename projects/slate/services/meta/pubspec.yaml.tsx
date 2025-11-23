import * as yaml from "jsr:@std/yaml";

export class ConfigFile {
    readonly #content?: string;
    readonly #path?: URL | string;

    constructor(content?: string) {
        this.#content = content;
    }

    public get path() {
        return this.#path;
    }

    //---
    public get content() {
        return this.#content ?? "string";
    }
    public set content(value: string) {
        this.#content = value;
    }
}

//---
import pubspec from "./pubspec.yaml" with { type: "text" };

//---
const $$yaml = new ConfigFile(pubspec);
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
