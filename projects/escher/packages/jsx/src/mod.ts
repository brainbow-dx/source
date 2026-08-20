// A JSX automatic-runtime that compiles JSX into `ScaffoldNode` trees instead of DOM/React
// elements — the shape `runtimes/web/src/description.rs`'s `ScaffoldDescription` deserializes
// (field-for-field: `content`, `styles`, `children`). Escher's `Scaffold` has no notion of tag
// names (no `div`/`span` distinction — every node is just content + styles + children), so every
// JSX element produces the same node shape regardless of its tag; the tag name itself is ignored.
//
// Use via a per-file pragma (`/** @jsxImportSource @escher/jsx */`) or a `deno.json` with
// `"compilerOptions": { "jsx": "react-jsx", "jsxImportSource": "@escher/jsx" }` — the latter is
// already set for this package's own `examples/`.

import type { Style } from "@escher/core/style";

export interface ScaffoldNode {
    content?: string;
    styles: Style[];
    children: ScaffoldNode[];
}

export interface ScaffoldProps {
    style?: Style | Style[];
    children?: JsxChild | JsxChild[];
}

export type JsxChild = ScaffoldNode | string | number | boolean | null | undefined;

export function jsx(_type: unknown, props: ScaffoldProps): ScaffoldNode {
    return buildNode(props);
}

export const jsxs = jsx;

export function Fragment(props: { children?: JsxChild | JsxChild[] }): ScaffoldNode {
    return buildNode(props);
}

function buildNode(props: { style?: Style | Style[]; children?: JsxChild | JsxChild[] }): ScaffoldNode {
    const styles = props.style == null ? [] : Array.isArray(props.style) ? props.style : [props.style];
    const rawChildren = props.children == null ? [] : Array.isArray(props.children) ? props.children : [props.children];

    const textParts: string[] = [];
    const children: ScaffoldNode[] = [];

    for (const child of rawChildren) {
        if (child == null || typeof child === "boolean") {
            continue;
        }
        if (typeof child === "string" || typeof child === "number") {
            textParts.push(String(child));
        } else {
            children.push(child);
        }
    }

    return {
        content: textParts.length > 0 ? textParts.join("") : undefined,
        styles,
        children,
    };
}

declare global {
    namespace JSX {
        // deno-lint-ignore no-empty-interface
        interface Element extends ScaffoldNode {}
        interface IntrinsicElements {
            [tag: string]: ScaffoldProps;
        }
    }
}
