import type { Surface } from "@brainbow/slate-core/surface";
import type { DrawFn } from "@brainbow/slate-core/draw";

export class LocalStore extends Map {
    construct() {
        //..
    }
}

// import * as c from "@brainbow/ethos/cwrap";

export class TerminalStore extends LocalStore {
    override construct() {
        //..
    }
}

// @c.struct("TerminalSurface")
export class TerminalSurface implements Surface {
    public readonly store = new TerminalStore();

    private _isRunning = true;

    constructor(
        private readonly backend: object,
    ) {
        //..
    }

    public get isRunning() {
        return this._isRunning;
    }

    public start() {
        this._isRunning = true;
    }

    public stop() {
        this._isRunning = false;
    }

    public async draw(drawFn: DrawFn) {
        const jsxTree = await drawFn(this);
        console.debug(`JSX Tree:\n`, jsxTree);
    }
}
