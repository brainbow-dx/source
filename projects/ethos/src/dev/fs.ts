import { resolve } from "@std/path";
import * as path from "@std/path";
import { exists, walk } from "@std/fs";
import type { WalkOptions } from "@std/fs";

//---
export interface CopyFilesOptions {
    walk?: WalkOptions;
    dryRun?: boolean;
    onMakeDir?: (path: string) => boolean | undefined | void;
    onCopy?: (contet: string | null, path: string) => boolean | undefined | void;
}

export async function copyFiles(from: string, to: string, options?: CopyFilesOptions) {
    const walkDir = resolve(from);
    const walkOptions = {
        ...options?.walk
    };

    for await (const dirEntry of walk(walkDir, walkOptions)) {
        // The relative path to the entrypoint.
        const entryRelativePath = dirEntry.path.replace(walkDir, `.`);
        const dirRelativePath = path.dirname(entryRelativePath);

        const fromPath = resolve(from, entryRelativePath);
        const toPath = resolve(to, entryRelativePath);

        if (dirEntry.isFile) {
            // TODO: Handle symlinks ..
            const content = await Deno.readTextFile(fromPath);
            const parentDir = path.dirname(toPath);

            const shouldCopy = options?.onCopy?.call(options, content, entryRelativePath);

            if (!options?.dryRun && shouldCopy !== false) {
                const shouldMakeDir = await exists(parentDir) == false
                    && options?.onMakeDir?.call(options, dirRelativePath);

                if (shouldMakeDir !== false) {
                    await Deno.mkdir(parentDir, { recursive: true });
                }

                await Deno.writeTextFile(toPath, content);
            }
        }
    }
}