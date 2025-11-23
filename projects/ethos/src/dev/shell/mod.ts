export * from "./cli.ts";

export { $ } from "dzx";
export * as dzx from "dzx";

export * from "@david/which";

//---
import { join } from "@std/path";
import { exists } from "@std/fs";

import { $ } from "dzx";

//---
export function $$(strings: TemplateStringsArray, ...expressions: any[]) {
    return $(strings, ...expressions);
};

export async function homedir(subDir: string, ensure?: boolean): Promise<string | undefined> {
    // TODO: What do we do on ios/android?
    const envHomeDir = Deno.build.os === "windows"
        ? Deno.env.get("USERPROFILE")
        : Deno.env.get("HOME");

    const homeDir = join(envHomeDir!, subDir);
    if (ensure && await exists(homeDir)) {
        await Deno.mkdir(homeDir, { recursive: true });
    }

    return homeDir;
}

// deno-lint-ignore no-explicit-any
export function banner(strings: TemplateStringsArray, ...values: any[]) {
    const output = strings.reduce((acc, currentString, i) => {
        acc += currentString + (values[i] || '');
        return acc;
    }, '');

    // Find the indentation of the first non-empty line
    const lines = output.split('\n');
    const indentMatch = lines.find(line => line.trim().length > 0)?.match(/^\s*/);
    const indent = indentMatch ? indentMatch[0].length : 0;

    // Remove that indentation from all lines
    return lines
        .filter(line => line.replaceAll(/\n/ig, ''))
        .map(line => line.substring(indent))
        .join('\n');
}