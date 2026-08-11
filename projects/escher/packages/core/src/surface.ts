import type { DrawFn } from "./draw.ts";

export interface Surface {
    draw(drawFn: DrawFn<Surface>): Promise<void> | void;
}

export enum SurfaceMode {
    Empty,
}