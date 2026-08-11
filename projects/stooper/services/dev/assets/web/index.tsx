// deno-lint-ignore-file
import * as uuid from "@std/uuid";

import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";

import { Workspace } from "@ethos/workspace";

import { SketchEditor, EditorStore, EditorStoreContext } from "@escher/sketch/components/editor";
import type { EditorOptions, Resource, SketchSceneData } from "@escher/sketch/components/editor";

import "#assets/web/app.css";
import "#assets/web/editor.css";

//---
function updateSettings(settings: object) {
    try {
        for (const [key, value] of Object.entries(settings)) {
            // TODO: Probably check and warn for malformed keys?
            localStorage.setItem(`sketch:${key}`, JSON.stringify(value));
        }
    } catch (error) {
        console.error("Failed to save sidebar state to localStorage", error);
    }
}

//---
export class ResourceEditorStore<S extends Partial<SketchSceneData>> implements EditorStore<S> {
    constructor(
        public readonly resource?: Resource,
        public readonly workspace?: Workspace,
        public readonly options: EditorOptions = {
            //..
        },
    ) {
        //..
    }
    
    public get resourceCacheKey() {
        return (this.resource)
            ? `resource:${this.resource.id}`
            : undefined;
    }
    
    public async mountWorkspace(): Promise<boolean> {
        try {
            // const workspaceHandleRecord = localStorage.getItem(`workspace.root-dir-handle`);
            
            // if (workspaceHandleRecord) {
            //     const workspaceDir = await navigator.storage.getDirectory();
            //     const handle = await workspaceDir.getFileHandle("TODO");
                
            //     const permissionStatus = await (handle as any).queryPermission({
            //         mode: "readwrite",
            //     });
                
            //     switch (permissionStatus) {
            //     case "granted":
            //         return true;
            //     }
            // }
            
            // const handle = await (window as any).showDirectoryPicker();
            // const permission = await (handle as any).requestPermission({ mode: "readwrite" });
            
            // console.log(`Root Dir:`, permission, handle);
            
            // localStorage.setItem(workspaceHandleKey, JSON.stringify(handle));
        } catch (error) {
            console.error("Failed to mount workspace:", error);
        }
        
        return false;
    }
    
    public async loadResource(): Promise<boolean> {
        throw new Error("Method not implemented.");
    }
    
    public async saveResource(): Promise<boolean> {
        throw new Error("Method not implemented.");
    }
    
    public getScene(): S | null {
        if (!this.resourceCacheKey) {
            return null;
        }
        
        let sceneData = null;
        try {
            const record = localStorage.getItem(this.resourceCacheKey);
            
            if (record) {
                sceneData = JSON.parse(record) as S;
                if (sceneData?.appState) {
                    sceneData.appState = {
                        ...sceneData.appState,
                        collaborators: new Map(),
                    };
                }
            }
        } catch (error) {
            console.error(`Failed to load scene data:`, error);
        }
        
        return sceneData;
    }
    
    public updateScene(sceneData: S): void {
        if (!this.resourceCacheKey) {
            return console.error(`Resource not available.`);
        }
        
        try {
            localStorage.setItem(this.resourceCacheKey, JSON.stringify(sceneData));
        } catch (error) {
            console.error(`Failed to update scene data:`, error);
        }
    }
}

//---
const editorRoot = document.getElementById("editor");

if (editorRoot && editorRoot.classList.contains("surface")) {
    const editor = new ResourceEditorStore();
    createRoot(editorRoot).render(
        <StrictMode>
            <EditorStoreContext.Provider value={[editor, undefined]}>
                <SketchEditor
                    onSettingsChange={updateSettings}
                />
            </EditorStoreContext.Provider>
        </StrictMode>,
    );
}