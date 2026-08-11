import { restoreAppState, serializeAsJSON } from "@excalidraw/excalidraw";
import type { AppState, ExcalidrawImperativeAPI, SceneData } from "@excalidraw/excalidraw/types";
import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";

import type { Resource } from "#src/resource.ts";

//--
import { exportToSvg } from "@excalidraw/excalidraw";
import * as xml from "@libs/xml";

const handleImport = async (file: any) => {
  const contents = await file.text();
  
  // TODO: We should use 
  const parser = new DOMParser();
  const svgDoc = parser.parseFromString(contents, "image/svg+xml");
  
  const metadataString = svgDoc.querySelector("metadata")?.textContent;
  if (metadataString) {
    // TODO: const data = JSON.parse(metadataString);
  }

  // Then load elements into Excalidraw as usual
};

const exportWithMetadata = async (excalidrawAPI: any, myGlobalData: any) => {
  const elements = excalidrawAPI.getSceneElements();
  const appState = excalidrawAPI.getAppState();
  const files = excalidrawAPI.getFiles();

  const svg = await exportToSvg({
    elements,
    appState,
    files,
    // The metadata property accepts a string. 
    // It is injected into the <metadata> section of the SVG.
    metadata: JSON.stringify({
      version: "1.0",
      customGlobalId: "doc_123",
      ...myGlobalData
    }),
  });

  return svg;
};

//--
// TODO: Move to `@brainbow/atlas/store` ..
export type Index = string | number;

// TODO: Move to `@brainbow/atlas/store` ..
export interface Key {
    toString(): string;
}

export interface StorageProvider {
    get<V>(path: Key | Key[]): V | undefined;
    set<V>(path: Key | Key[], record: V): void;
}

// TODO: Move to `@brainbow/atlas/store @ src/store/backend/web.ts` ..
export class WebStorageProvider implements StorageProvider {
    constructor(
        public readonly storage?: Storage,
    ) {
        //..
    }
    
    get<V>(path: Key[]) {
        const record = this.storage?.getItem(path.join(':'));
        return (record)
            ? JSON.parse(record) as V
            : undefined;
    }
    
    // TODO: We should return the previous value (if any).
    set<V>(path: Key[], value: V): Promise<void> | void {
        if (value != undefined) {
            this.storage?.setItem(path.join(':'), JSON.stringify(value));
        }
    }
}

// TODO: Move to `@brainbow/atlas/store @ src/store.ts` ..
export interface StoreOptions {
    namespace: string[];
}

// TODO: Move to `@brainbow/atlas/store @ src/store.ts` ..
export abstract class Store implements StorageManager {
    constructor(
        public readonly storage: StorageProvider,
        public readonly options: StoreOptions = {
            namespace: [],
        },
    ) {
        //..
    }
    
    public estimate(): Promise<StorageEstimate> {
      throw new Error("Method not implemented.");
    }
    
    public getDirectory(): Promise<FileSystemDirectoryHandle> {
      throw new Error("Method not implemented.");
    }
    
    public persist(): Promise<boolean> {
      throw new Error("Method not implemented.");
    }
    
    public persisted(): Promise<boolean> {
      throw new Error("Method not implemented.");
    }
}

//---
export interface SceneStoreOptions extends StoreOptions {
    //..
}

export interface SceneStore {
    // new (testValue: number): SceneStore;
    getSceneData(): any;
    updateSceneData(_: any): any;
}

export class ExcalidrawSceneStore extends Store implements SceneStore {
    constructor(
        // TODO: A real UUID type pls ..
        public readonly uuid: string,
        public override readonly storage: StorageProvider,
        public override readonly options: SceneStoreOptions = {
            namespace: [],
        },
    ) {
        super(storage, Object.assign(options, {
            namespace: ["scene", ...options.namespace],
        }));
    }
    
    public get namespace() {
        return this.options.namespace;
    }
    
    private getKey(key: string[]) {
        return [...this.namespace, this.uuid, ...key];
    }
    
    //--
    public getSceneData() {
        const elements = this.getElements();
        const appState = this.getAppState();
        return { elements, appState };
    }
    
    public getElements() {
        // TODO: restoreElements(elements);
        return this.storage.get<ExcalidrawElement[]>(this.getKey([
            "elements", // TODO: Get this from metadata.
        ]));
    }
    
    public getAppState() {
        const appState = this.storage.get<AppState>(this.getKey([
            "app-state", // TODO: Get this from metadata.
        ]));
        
        if (appState) {
            // JSON-stringifying drops type data, do we need to re-
            // Note: Probably don't need to do this when using indexedDb ..
            Object.assign(appState, {
                // TODO: Keep previously listed collaborators.
                collaborators: new Map(),
            });
            
            restoreAppState(appState, {
                //..
            });
        }
        
        return appState;
    }
    
    public setElements(elements: ExcalidrawElement[]) {
        // TODO: Validate and clean elements ..
        this.storage.set<ExcalidrawElement[]>([
            ...this.options.namespace,
            this.uuid,
            "elements",
        ], elements.map(element => {
            return Object.assign(element, {
                customData: {
                    ...element.customData,
                    test: "what??",
                }
            })
        }));
    }
    
    public setAppState(appState: Partial<AppState>) {
        const key = this.getKey(["app-state"]);
        const record = {
            ...appState,
            selectedElementIds: undefined,
            activeEmbeddable: undefined,
            // TODO: What else??
        };
        
        // TODO: Use `serializeAsJSON(elements, appState, "database")` ..
        
        this.storage.set<Partial<AppState>>(key, record);
    }
    
    public updateSceneData({ elements, appState }: SceneData) {
        if (elements && elements.length > 0) {
            this.setElements([...elements]);
        }
        
        if (appState) {
            this.setAppState(appState);
        }
    }
}
