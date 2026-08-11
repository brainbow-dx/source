// deno-lint-ignore-file
import * as uuid from "@std/uuid/v5";
import { NAMESPACE_DNS, NAMESPACE_URL } from "@std/uuid";

import { createContext, Dispatch, SetStateAction, StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";

import * as xml from "@libs/xml";

import { Workspace } from "@ethos/core/workspace";

import { MutableState } from "#src/external/react/state.ts";

import { ExcalidrawSceneStore, SceneStore, WebStorageProvider } from "#src/stores/scene/mod.ts";

import { Editor, EditorOptions, EditorContext } from "#src/components/editor/mod.ts";
import { SketchEditor } from "#src/components/editor/mod.ts";

import { Resource } from "#src/resource.ts";

// TODO: import "@excalidraw/excalidraw/index.css";
import "#assets/draw.css";

async function setSvgFavicon(svgSource: string, size = 64) {
    try {
        const svgDataUrl = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgSource)}`;
        console.debug(`svgDataUrl:`, svgDataUrl);
        console.warn(`TODO: Set favicon to svg url!`);
        /*
        return new Promise((resolve, reject) => {
            try {
                const tmpImage = new Image();
                tmpImage.src = svgDataUrl;
                
                tmpImage.onload = () => {
                    const tmpCanvas = document.createElement("canvas");
                    tmpCanvas.width = size;
                    tmpCanvas.height = size;

                    const tmpCanvas2dContext = tmpCanvas.getContext("2d");
                    if (tmpCanvas2dContext != undefined) {
                        tmpCanvas2dContext.clearRect(0, 0, size, size);
                        tmpCanvas2dContext.drawImage(tmpImage, 0, 0, size, size);
                    }

                    const pngDataUrl = tmpCanvas.toDataURL("image/png");

                    let favicon = document.querySelector("link[rel~=\"icon\"]");
                    if (!favicon) {
                        favicon = document.createElement("link");
                        favicon.setAttribute("rel", "icon");
                        document.head.appendChild(favicon);
                    }
                    
                    favicon.setAttribute("type", "image/png");
                    favicon.setAttribute("href", pngDataUrl);

                    resolve(pngDataUrl);
                };
                
                tmpImage.onerror = (errorEvent) => {
                    console.warn(`Failed to build image from SVG!`);
                    reject(errorEvent);
                };
            } catch (error) {
                console.warn(`Failed to load canvas for iamge:`, error);
            }
        });
        */
    } catch (error) {
        console.error(`Failed to set SVG Favicon:`, error);
    }
}

//---
export interface ResourceResourceOptions {
    autoload: boolean;
    encoder: TextEncoder;
}

export class ResourceRecord implements Resource {
    #handle?: FileSystemHandle;
    #encoder = new TextEncoder();
    
    constructor(
        public readonly path: URL | string,
        public readonly uuid: string,
        public readonly options: Readonly<ResourceResourceOptions> = {
            encoder: new TextEncoder(),
            autoload: false,
        },
    ) {
        //..
    }
    
    public get name() {
        return undefined;
    }
    
    public get handle() {
        return this.#handle;
    }
}

export enum ResourceEditorState {
    unknown = 0,
    loading = 1,
    ready = 2,
    unloading = 3,
}

export class ResourceEditor implements Editor {
    // TODO: We want the scene store itself to be generic.
    // private scene: ExcalidrawSceneStore;
    
    #state = ResourceEditorState.unknown;
    #broadcast: BroadcastChannel;
    
    constructor(
        public readonly workspace: Workspace,
        public readonly resource: Resource,
        private readonly sceneStore?: SceneStore,
        public readonly options: EditorOptions = {
            //..
        },
    ) {
        // TODO: We should wrap this in a broadcast controller/broker so
        //  we can reuse it in the resource embed location.
        this.#broadcast = new BroadcastChannel(`resource:${this.resource.path}`);
        this.#broadcast.addEventListener("message", this.onBroadcastMessage);
    }
    
    public get state() {
        return this.#state;
    }
    
    private get scene() {
        return this.sceneStore;
    }
    
    public async mount(): Promise<boolean> {
        if (this.state == ResourceEditorState.loading) {
            return false;
        }
        
        if (!this.resource) {
            throw new Error(`resource not set`);
        }
        
        try {
            // TODO: if (options.handleGlobalEvents == true)
            window.addEventListener("message", this.onEditorMessage);
            
            // TODO:
            //  1. Attempt to get the cached scene from scene store.
            //      a. 
            //  2. Attempt to get the Workspace handle from db
            
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
            this.#state = ResourceEditorState.ready;
            
            console.debug(`Mounted ResourceEditor!`);
            
            this.#broadcast.postMessage({
                message: `Resource Editor Ready!`,
            });
        } catch (error) {
            console.error("Failed to mount workspace:", error);
        }
        
        return this.state == ResourceEditorState.ready;
    }
    
    private async onEditorMessage(event?: MessageEvent) {
        if (event?.data.source?.toString().startsWith("react-")) {
            return; // void console.debug(`Skip react messages ..`);
        }
        
        console.log(`Window Message:`, event?.data.source, event);
    }
    
    private async onBroadcastMessage(event?: any) {
        console.log(`Broadcast Message:`, event);
    }
    
    public getSceneData() {
        return this.scene?.getSceneData();
    }
    
    public updateSceneData(sceneData: any) {
        return this.scene?.updateSceneData(sceneData);
    }
    
    public updateSettings(settings: object) {
        try {
            for (const [key, value] of Object.entries(settings)) {
                // TODO: Probably check and warn for malformed keys?
                localStorage.setItem(`sketch:${key}`, JSON.stringify(value));
            }
        } catch (error) {
            console.error("Failed to save sidebar state to localStorage", error);
        }
    }
    
    public async setEditorIcon(iconSource: string) {
        return await setSvgFavicon(iconSource);
    }
}

//---
const editorElement = document.getElementById("editor");

if (editorElement) {
    const storage = new WebStorageProvider(localStorage);
    
    // TODO: Get root from runtime/config.
    const workspace = new Workspace("/");
    
    // TODO: Get `Resource` instances from the workspace.
    const workspaceResourceEncoder = new TextEncoder();
    const resourcePath = workspaceResourceEncoder.encode(`${window.location.origin}${window.location.pathname}`);
    const resourceId = await uuid.generate(NAMESPACE_DNS, resourcePath);
    const resource = new ResourceRecord(window.location.pathname, resourceId);
    
    // TODO: Get the correct backend for the current runtime.
    const scene = new ExcalidrawSceneStore(resource.uuid, storage, {
        namespace: [],
    });
    
    const editor = new ResourceEditor(workspace, resource, scene);
    
    await editor.setEditorIcon(`
        <?xml version="1.0" encoding="UTF-8" ?>
        <svg width="64px"
            height="64px"
            viewBox="0 0 24 24"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
            stroke="#000000"
            data-darkreader-inline-stroke=""
            style="--darkreader-inline-stroke: var(--darkreader-text-000000, #e8e6e3);">
            <g id="SVGRepo_bgCarrier"
            stroke-width="0" />
            <g id="SVGRepo_tracerCarrier"
            stroke-linecap="round"
            stroke-linejoin="round" />
            <g id="SVGRepo_iconCarrier">
                <path d="M4 12H4.01M8 12H8.01M16 12H16.01M12 12H12.01M20 12H20.01M8.5 4H7.2C6.0799 4 5.51984 4 5.09202 4.21799C4.71569 4.40973 4.40973 4.71569 4.21799 5.09202C4 5.51984 4 6.0799 4 7.2V8.5M15.5 4H16.8C17.9201 4 18.4802 4 18.908 4.21799C19.2843 4.40973 19.5903 4.71569 19.782 5.09202C20 5.51984 20 6.07989 20 7.2V8.5M20 15.5V16.8C20 17.9201 20 18.4802 19.782 18.908C19.5903 19.2843 19.2843 19.5903 18.908 19.782C18.4802 20 17.9201 20 16.8 20H15.5M4 15.5V16.8C4 17.9201 4 18.4802 4.21799 18.908C4.40973 19.2843 4.71569 19.5903 5.09202 19.782C5.51984 20 6.0799 20 7.2 20H8.5"
                    stroke="#ffffff"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    data-darkreader-inline-stroke=""
                    style="--darkreader-inline-stroke: var(--darkreader-text-ffffff, #e8e6e3);" />
            </g>
        </svg>
    `);
    
    createRoot(editorElement).render(
        <StrictMode>
            <EditorContext.Provider value={[editor, undefined]}>
                <SketchEditor
                    // TODO: Probably want SketchEditor itself to
                    //  to do the direct editor backend update..
                    onSettingsChange={editor.updateSettings}
                />
            </EditorContext.Provider>
        </StrictMode>,
    );
}