// Turns a Scaffold `ScaffoldDescription` (see `escher/runtimes/web/src/description.rs` for the
// canonical Rust shape this mirrors) into Unity UI Toolkit UXML + USS text. Only the property
// subset Escher's own terminal/web surfaces already render (`backgroundColor`/`size`) is covered
// — see `spec/agents/proposals/uxml-uss-codegen.md` for why. Reusable/importable, not one-off
// script content — `shape-demo.ts` in this same directory is the one-off part.
//
// Plain JS syntax despite the `.ts` extension: `ethos-deno` (`packages/deno`) runs `.ts` files
// directly via V8 with no TypeScript-stripping pass (confirmed live — real type annotations
// threw `SyntaxError: Missing initializer in const declaration`, since nothing here erases them
// before the module reaches V8). Types are documented via JSDoc instead. See
// `escher/spec/ROADMAP.md`'s M4 for the tracked follow-up to add real stripping.

/**
 * @typedef {{ content?: string, styles?: StyleDescription[], children?: ScaffoldDescription[] }} ScaffoldDescription
 * @typedef {{ type: "backgroundColor", color: string } | { type: "size", width?: ValueDescription, height?: ValueDescription } | { type: string, [key: string]: unknown }} StyleDescription
 * @typedef {{ unit: "auto" } | { unit: "px", value: number } | { unit: "percent", value: number } | { unit: "fill", value: number }} ValueDescription
 * @typedef {{ uxml: string, uss: string }} UxmlOutput
 */

/**
 * @param {ValueDescription | undefined} value
 * @returns {string | undefined}
 */
function valueToUss(value) {
    if (!value) return undefined;
    switch (value.unit) {
        case "auto":
            return "auto";
        case "px":
            return `${value.value}px`;
        case "percent":
            return `${value.value}%`;
        // USS has no direct "fill" analogue (that's an Escher/flex-grow concept) — 100% is the
        // closest static-generation equivalent for a single-child container.
        case "fill":
            return "100%";
        default:
            return undefined;
    }
}

/**
 * @param {ScaffoldDescription} description
 * @returns {string[]}
 */
function rulesForNode(description) {
    const rules = [];
    for (const style of description.styles ?? []) {
        switch (style.type) {
            case "backgroundColor":
                rules.push(`background-color: ${style.color};`);
                break;
            case "size": {
                const width = valueToUss(style.width);
                const height = valueToUss(style.height);
                if (width) rules.push(`width: ${width};`);
                if (height) rules.push(`height: ${height};`);
                break;
            }
            default:
                // Unhandled property — see the proposal doc's scope note. Silently skipped,
                // same "graceful unsupported subset" behavior `escher-web`'s own CSS mapping
                // already has for Border/Heading/etc.
                break;
        }
    }
    return rules;
}

/**
 * @param {ScaffoldDescription} description
 * @returns {string}
 */
function elementTagFor(description) {
    // Tonight's demo only ever emits plain containers/labels — `ui:Button` support can follow
    // the same pattern once Escher actually needs it through this path.
    return description.content !== undefined ? "ui:Label" : "ui:VisualElement";
}

/**
 * @param {string} value
 * @returns {string}
 */
function escapeXml(value) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
}

/**
 * @param {ScaffoldDescription} description
 * @param {string[]} uxmlLines
 * @param {string[]} ussLines
 * @param {string} indent
 * @param {{ value: number }} counter
 */
function walk(description, uxmlLines, ussLines, indent, counter) {
    counter.value += 1;
    const className = `escher-node-${counter.value}`;
    const tag = elementTagFor(description);
    const rules = rulesForNode(description);

    if (rules.length > 0) {
        ussLines.push(`.${className} {`);
        for (const rule of rules) ussLines.push(`    ${rule}`);
        ussLines.push(`}`);
    }

    const attrs = [`class="${className}"`];
    if (description.content !== undefined) {
        attrs.push(`text="${escapeXml(description.content)}"`);
    }

    const children = description.children ?? [];
    if (children.length === 0) {
        uxmlLines.push(`${indent}<${tag} ${attrs.join(" ")} />`);
        return;
    }

    uxmlLines.push(`${indent}<${tag} ${attrs.join(" ")}>`);
    for (const child of children) {
        walk(child, uxmlLines, ussLines, indent + "    ", counter);
    }
    uxmlLines.push(`${indent}</${tag}>`);
}

// Unverified against a real Unity import tonight (see the proposal doc) — written against
// UI Toolkit's documented UXML/USS schema: `ui:UXML` root with the `UnityEngine.UIElements`
// namespace, `<Style src="...">` referencing the sibling `.uss` by relative path.
/**
 * @param {ScaffoldDescription} description
 * @param {string} ussFileName
 * @returns {UxmlOutput}
 */
export function scaffoldDescriptionToUxml(description, ussFileName) {
    const uxmlLines = [];
    const ussLines = [];
    walk(description, uxmlLines, ussLines, "        ", { value: 0 });

    const uxml =
        [
            `<ui:UXML xmlns:ui="UnityEngine.UIElements" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">`,
            `    <Style src="${ussFileName}" />`,
            `    <ui:VisualElement class="escher-root">`,
            ...uxmlLines,
            `    </ui:VisualElement>`,
            `</ui:UXML>`,
        ].join("\n") + "\n";

    const uss = ussLines.join("\n") + "\n";

    return { uxml, uss };
}
