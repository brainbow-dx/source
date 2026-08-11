import type { Surface } from "@escher/core/surface";
import type { DrawFn } from "@escher/core/draw";

export class LocalStore extends Map {
    construct() {
        //..
    }
}

// import * as c from "@ethos/cwrap";

export class TerminalStore extends LocalStore {
    override construct() {
        //..
    }
}

// deno-lint-ignore no-empty-interface
export interface TerminalSurfaceProps {
    //..
}

// @c.struct("TerminalSurface")
export class TerminalSurface implements Surface {
    public readonly store: TerminalStore = new TerminalStore();

    constructor(
        private readonly backend?: object,
    ) {
        //..
    }

    #isRunning = true;
    
    public get isRunning(): boolean {
        return this.#isRunning;
    }

    public async draw(drawFn: DrawFn<TerminalSurface>): Promise<void> {
        const jsxTree = await drawFn(this);
        console.debug(`JSX Tree:\n`, jsxTree);
    }
}

/**
 * Draw a new surface in the current runtime's primary surface.
 * @param drawFn A function to use to draw the surface.
 * @returns void
 */
export async function draw(drawFn: DrawFn<TerminalSurface>): Promise<void> {
    // Todo: Set this on `globalThis` ..
    const terminalSurface = new TerminalSurface();
    return await terminalSurface.draw(drawFn);
}