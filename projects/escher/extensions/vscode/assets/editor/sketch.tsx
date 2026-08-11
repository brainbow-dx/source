// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { exportToSvg, exportToBlob, hashElementsVersion, loadFromBlob } from "@excalidraw/excalidraw";
import type { AppState, BinaryFiles, ExcalidrawInitialDataState } from "@excalidraw/excalidraw/types";
import type { NonDeletedExcalidrawElement } from "@excalidraw/excalidraw/element/types";

import { SketchEditor } from "@escher/web/components/editor";

import "./sketch.css";

//---
const preview = document.getElementById("preview");

//---
// @ts-ignore
export const vscode = acquireVsCodeApi();

//---
async function openResource(element: { link: string }) {
  console.debug(`Opening '${element.link}' ..`);
  vscode.postMessage({
    kind: "request:com.brainbow.sketch.openResource",
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
      elements,
      appState: {
        ...appState,
        exportDarkMode: true,
        exportBackground: false,
        exportEmbedScene: true,
      },
      files,
      renderEmbeddables: false,
      skipInliningFonts: false,
      reuseImages: true,
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
const svgBlob = new Blob([preview?.innerHTML!], { type: "image/svg+xml" });
let svgData: ExcalidrawInitialDataState | undefined = undefined;
try {
  svgData = await loadFromBlob(svgBlob, null, null);
  console.debug(`SVG Data:`, svgData);
} catch (error: any) {
  // TODO: IMAGE_NOT_CONTAINS_SCENE_DATA
  if (error.code === "IMAGE_NOT_CONTAINS_SCENE_DATA") {
    console.warn(`TODO: Try to inject svg directly into a new scene ..`);
    vscode.postMessage({
      kind: "request:logDebugMessage",
      summary: `TODO: Try to inject svg directly into a new scene ..`,
      error: JSON.stringify(error), // TODO: Wrap with metadata ..
    });
  } else {
    vscode.postMessage({
      kind: "event:onError",
      summary: `Failed to load scene from SVG.`,
      error: JSON.stringify(error), // TODO: Wrap with metadata ..
    });
  }
}

const root = document.getElementById("root");
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
  createRoot(root)
    .render(
      <StrictMode>
        <SketchEditor
          displayName="Sketch"
          onOpenResource={openResource}
          onSceneReady={() => console.debug(`Scene ready!`)}
          onChange={saveScene}
          svgData={svgData}
        />
      </StrictMode>,
    );
}