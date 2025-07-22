export * from "./cli.ts";

export * from "dzx";
export * from "@david/which";

//---
import { resolve } from "@std/path";
import { exists } from "@std/fs";

export async function homedir(suffix: string, ensure?: boolean): Promise<string | undefined> {
    // TODO: What do we do on ios/android?
    const envHomeDir = (Deno.build.os === "windows")
        ? Deno.env.get("USERPROFILE")
        : Deno.env.get("HOME");

    const userDir = resolve(envHomeDir!, suffix);
    if (ensure && await exists(userDir)) {
        await Deno.mkdir(userDir, { recursive: true });
    }

    return userDir;
}