export type { Args } from "jsr:@std/cli";

//--
import { parseArgs } from "jsr:@std/cli";
import type { Args } from "jsr:@std/cli";

import { $ } from "dzx";

/**
 * Mount a cli application for 
 * @param args Set of string arguments to be parsed.
 * @returns Parsed arguments or nothing.
 */
export function parse<A extends Args>(args: string[] = [], setupFn?: (args: A) => void) {
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
