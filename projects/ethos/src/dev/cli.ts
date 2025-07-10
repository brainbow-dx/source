import { parseArgs } from "jsr:@std/cli";

import { $ } from "dzx";

/**
 * Mount a cli application for 
 * @param args Set of string arguments to be parsed.
 * @returns Parsed arguments or nothing.
 */
// deno-lint-ignore no-explicit-any
export function parseCli<A extends Record<string, any>>(args: string[] = []) {
    $.verbose = 2;
    $.shell = "bash";
    $.stdout = "inherit";
    $.stderr = "inherit";
    $.stdin = "inherit";

    return parseArgs<A>(args);
}
