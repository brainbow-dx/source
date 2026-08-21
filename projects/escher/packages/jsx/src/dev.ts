// The dev-mode JSX runtime entry point (`jsxImportSource`'s `/jsx-dev-runtime`, used by Deno's
// transpiler for non-production builds). No debug/owner tracking to add on top of `jsx` — the
// underlying `ScaffoldNode` tree carries no component identity for devtools to hook into — so
// this just forwards to the same builder, ignoring the extra dev-only arguments.

import { jsx } from "./mod.ts";

export { Fragment } from "./mod.ts";

export function jsxDEV(type: unknown, props: Parameters<typeof jsx>[1]): ReturnType<typeof jsx> {
    return jsx(type, props);
}
