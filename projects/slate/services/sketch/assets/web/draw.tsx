// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import SketchEditor from "./views/editor/sketch.tsx";

import "./draw.css";

function openResource(element: any) {
    // console.warn("Function not implemented.");
}

function loadScene() {
    // console.warn("Function not implemented.");
}

function updateDocument(changeEvent: any) {
    // console.warn("Function not implemented.");
}

//---
const root = document.getElementById("root");
if (root) {
    createRoot(root).render(
        <StrictMode>
            <SketchEditor
                $store={undefined}
                displayName="Sketch"
                loadSketch={true}
                saveSketch={true}
                onOpenResource={openResource}
                onSceneReady={loadScene}
                onChange={updateDocument}
            />
        </StrictMode>,
    );
}