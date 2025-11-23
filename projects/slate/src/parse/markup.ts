type Document = globalThis.Document;
type Element = globalThis.Element;
type Node = globalThis.Node;

let DOMParser: { new(): globalThis.DOMParser; };

if (typeof window !== 'undefined' && window.DOMParser) {
    DOMParser = window.DOMParser;
    console.log('Using native DOMParser (Browser environment)');
} else {
    const { DOMParser: DenoDOMParser } = await import('jsr:@b-fuze/deno-dom');
    DOMParser = DenoDOMParser;
    console.log('Using deno-dom DOMParser (Deno runtime)');
}

/**
 * Parses an HTML string into a Document object.
 * @param htmlString The HTML content as a string.
 * @returns The parsed Document object.
 */
export function parseMarkup(htmlString: string): Document {
    const parser = new DOMParser();
    // The 'text/html' mime type is standard for HTML parsing
    return parser.parseFromString(htmlString, 'text/html');
}

//---
export type { Document, Element, Node };