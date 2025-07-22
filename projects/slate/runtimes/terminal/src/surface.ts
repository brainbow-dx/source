import * as c from "@brainbow/cwrap";

import type { JSX } from "react";

type DrawJsx = (props: object) => JSX.Element;

interface Surface {
    draw(drawFn: DrawJsx): void;
}

@c.struct("TerminalSurface")
export class TerminalSurface implements Surface {
    constructor() {
        //..
    }

    draw(drawFn: DrawJsx) {
        const jsxTree = drawFn(this);
        console.debug(`JSX Tree:\n`, jsxTree);
    }
}
