// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { exportToSvg, exportToBlob, hashElementsVersion, loadFromBlob } from "@excalidraw/excalidraw";
import type { AppState, BinaryFiles, ExcalidrawInitialDataState } from "@excalidraw/excalidraw/types";
import type { NonDeletedExcalidrawElement } from "@excalidraw/excalidraw/element/types";

import Workspace from "@brainbow/slate-sketch/views/workspace";

import "./document.css";

//---
const preview = document.getElementById("preview");

//---
// @ts-ignore
export const vscode = acquireVsCodeApi();

//---
async function openResource(
    element: { link: string }
) {
    console.debug(`Opening '${element.link}' ..`);
    vscode.postMessage({
        kind: "request:openResource",
        element: element,
    });
};

// TODO: Store the scene hash in the preview element's data attrs ..
let lastKnownSceneHash: number | undefined = undefined;
async function saveScene(
    elements: readonly NonDeletedExcalidrawElement[],
    appState: Partial<AppState>,
    files: BinaryFiles
) {
    const sceneHash = hashElementsVersion(elements);
    if (!lastKnownSceneHash) {
        lastKnownSceneHash = sceneHash;
    }
    if (sceneHash != lastKnownSceneHash) {
        lastKnownSceneHash = sceneHash;

        console.debug(`Saving Scene From Parts:`, { elements, appState, files });
        const svg = await exportToSvg({
            appState: {
                ...appState,
                exportDarkMode: true,
                exportBackground: false,
                exportEmbedScene: true,
            },
            elements,
            files,
            renderEmbeddables: true,
            reuseImages: true,
            skipInliningFonts: false,
        });

        console.debug(`SVG Snapshot:`, svg);
        vscode.postMessage({
            kind: "event:onChange",
            content: svg.outerHTML,
        });
    }
};

//---
// TODO: Render loading state ..
// const svgBlob = new Blob([preview?.innerHTML!], { type: "image/svg+xml" });
// let svgData: ExcalidrawInitialDataState | undefined = undefined;
// try {
//     svgData = await loadFromBlob(svgBlob, null, null);
//     console.debug(`SVG Data:`, svgData);
// } catch (error: any) {
//     // TODO: IMAGE_NOT_CONTAINS_SCENE_DATA
//     if (error.code === "IMAGE_NOT_CONTAINS_SCENE_DATA") {
//         console.warn(`TODO: Try to inject svg directly into a new scene ..`);
//         vscode.postMessage({
//             kind: "request:logDebugMessage",
//             summary: `TODO: Try to inject svg directly into a new scene ..`,
//             error: JSON.stringify(error), // TODO: Wrap with metadata ..
//         });
//     } else {
//         vscode.postMessage({
//             kind: "event:onError",
//             summary: `Failed to load scene from SVG.`,
//             error: JSON.stringify(error), // TODO: Wrap with metadata ..
//         });
//     }
// }

const overlay = document.getElementById("overlay");
const contentDisplay = document.getElementById("root");
const codeEditor = document.getElementById("code");

if (!root) {
    console.warn(`No #root element found!`);
    vscode.postMessage({
        kind: "event:onError",
        summary: `Couldn't find root element.`,
    });
} else {
    vscode.postMessage({
        kind: "request:showInfoMessage",
        summary: `Loaded Sketch for svg! <3`,
    });

    if (codeEditor && contentDisplay) {
        // const { contentDocument, contentWindow } = contentDisplay;
        // const displayDocument = contentDocument || contentWindow.document;

        // displayDocument.documentElement.style.setProperty("color-scheme", "dark");
        // displayDocument.documentElement.setAttribute("contenteditable", "true");

        // displayDocument.open();
        // displayDocument.write(codeEditor.value);
        // displayDocument.close();

        // contentDisplay.innerHTML = `
        //     ${codeEditor.value}
        // `;
    }

    // TODO: Conditionally render layout/drawing tools.
    // createRoot(overlay)
    //     .render(
    //         <StrictMode>
    //             <Workspace
    //                 displayName="Sketch"
    //                 onOpenResource={openResource}
    //                 onSceneReady={() => console.debug(`Scene ready!`)}
    //                 onChange={saveScene}
    //                 svgData={svgData}
    //             />
    //         </StrictMode>,
    //     );
}