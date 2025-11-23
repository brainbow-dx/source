import type { DrawFn } from "./draw.ts";

export interface Surface {
    draw(drawFn: DrawFn): void;
}

export enum SurfaceMode {
    Empty,
}