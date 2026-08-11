// deno-lint-ignore-file

// TODO: As of Nov. 15 2025, `deno bundle --platform web` seems
//   to fail when using @std/path. Maybe misconfigured?
// import { join } from "@std/path";

import { join } from "@std/path";
import { debounce } from "@std/async";

import React, { useContext, createContext, useState, useEffect, useRef, useCallback, ReactElement, useMemo, SetStateAction, Dispatch, RefObject } from "react";

import { useLocalStorage } from "@uidotdev/usehooks";

import { CaptureUpdateAction, convertToExcalidrawElements, Excalidraw, exportToSvg, Footer, MainMenu, mutateElement, newElementWith, Sidebar, useHandleLibrary, WelcomeScreen } from "@excalidraw/excalidraw";
import { MIME_TYPES, loadSceneOrLibraryFromBlob } from "@excalidraw/excalidraw";
import type { ImportedLibraryData } from "@excalidraw/excalidraw/data/types";
import type { ExcalidrawElement, ExcalidrawEmbeddableElement, NonDeleted, NonDeletedExcalidrawElement, OrderedExcalidrawElement, Theme } from "@excalidraw/excalidraw/element/types";
import type { ActiveTool, AppState, BinaryFiles, Collaborator, ExcalidrawImperativeAPI, ExcalidrawInitialDataState, SceneData, SidebarName, SidebarTabName, SocketId, UIAppState, UIOptions } from "@excalidraw/excalidraw/types";

import { Workspace } from "@ethos/core/workspace";
// import { openResource, loadSceneOrLibrary } from "@escher/sdk/utils";
// import { fromLocalStorage } from "@escher/sdk/utils";

import { EditorToolsOverlay } from "#src/components/editor/tools.tsx";
import { ElementControlPanel } from "./sidebar/panels/controls.tsx";

import { Resource } from "#src/resource.ts";
import { ResourceEmbed } from "#src/components/editor/resource/embed.tsx";
import { ResourceDisplayOverlay } from "./resource/overlay.tsx";

import { SceneOverviewPanel } from "./sidebar/scene/overview.tsx";
import { ElementInspectorPanel } from "./sidebar/panels/inspect.tsx";

// declare global {
//     var EXCALIDRAW_ASSET_PATH: string;
//     var SKETCH_ASSET_PATH: string | undefined;
// }

export type FullExcalidrawElement = {
  [K in keyof ExcalidrawElement]: ExcalidrawElement[K];
};

export type NativePointerEvent = CustomEvent<{
    nativeEvent: MouseEvent | React.PointerEvent<HTMLCanvasElement>;
}>;

export type MutableState<T> = [
    T | undefined,
    Dispatch<SetStateAction<T | undefined>> | undefined,
];

//---
export interface Editor {
    resource?: Resource;
    workspace?: Workspace;
    options: EditorOptions;
    mount(): Promise<boolean> | boolean;
    getSceneData(): SceneData;
    updateSceneData(sceneData: SceneData): void;
}

export type EditorTheme = "light" | "dark";

export interface EditorOptions {
    theme?: EditorTheme;
}

export const EditorContext: React.Context<MutableState<Editor>> = createContext<MutableState<Editor>>([
    undefined,
    undefined,
]);

//--
// deno-lint-ignore no-slow-types
export const ExcalidrawContext = createContext<RefObject<ExcalidrawImperativeAPI | undefined> | undefined>(undefined);

// deno-lint-ignore no-slow-types
export function useExcalidraw() {
    const context = useContext(ExcalidrawContext);

    if (context === undefined) {
        throw new Error("useExcalidraw must be used within an ExcalidrawProvider");
    }

    return context;
}

// type ExcalidrawUpdateFn = (elements: any, appState: AppState, files: any) => void;

export interface ActiveEmbeddable {
    element: NonDeletedExcalidrawElement;
    state: "hover" | "active";
}

export interface ActiveSidebar {
    name: SidebarName;
    tab?: SidebarTabName;
}

export type LockScrollPosition = [number, number, number];

export interface SketchEditorErrorState {
    hasError: boolean;
    message?: string;
}

export class SketchEditorErrorBoundary extends React.Component<React.PropsWithChildren, SketchEditorErrorState> {
    public override readonly state = {
        hasError: false,
        message: undefined,
    }
    
    static getDerivedStateFromError(error: any) {
        return {
            hasError: true,
            message: error,
        }
    }
    
    override componentDidCatch(error: any, errorInfo: any) {
        console.error(`ResourceEditorError:`, error, errorInfo);
    }
    
    override render() {
        if (this.state.hasError == false) {
            return this.props.children; 
        }

        return (
            <dialog role="alertdialog" className="editor-error">
                <h1>Error!</h1>
                <p>{JSON.stringify(this.state.message, null, 2)}</p>
            </dialog>
        )
    }
}

export interface SketchEditorProps {
    onSettingsChange: (settings: object) => void,
}

export function SketchEditor(props: SketchEditorProps) {
    const [editor, setEditor] = useContext(EditorContext);
    const excalidrawApi = useRef<ExcalidrawImperativeAPI>(undefined);
    
    //--
    const [activeTheme, setActiveTheme] = useState<EditorTheme>(editor?.options.theme ?? "dark");
    const [embedMode, setEmbedMode] = useState(window !== window.top);
    const [viewModeEnabled, setViewModeEnabled] = useState(window !== window.top);
    const [zenModeEnabled, setZenModeEnabled] = useState(window !== window.top);
    const [gridModeEnabled, setGridModeEnabled] = useState(window === window.top);
    
    const [activeEmbeddable, setActiveEmbeddable] = useState<ActiveEmbeddable | null>(null);
    const [selectedGroupIds, setSelectedGroupIds] = useState<{[key: string]: boolean}>({});
    const [selectedElementIds, setSelectedElementIds] = useState<{[key: string]: boolean}>({});
    
    //--
    const [lockScrollPos, setLockScrollPos] = useState<LockScrollPosition | null>(null);
    const [sidebarDocked, setSidebarDocked] = useState<boolean>(false);
    const [openSidebar, setOpenSidebar] = useState<ActiveSidebar | undefined>(undefined);
    
    const [activeTool, setActiveTool] = useState<ActiveTool | undefined>(undefined);
    const [isAnimating, setIsAnimating] = useState<boolean>(false);
    
    useMemo(() => {
        editor?.mount();
        
        return () => {
            // TODO: Unmount editor??
        }
    }, [
        //..
        editor,
        excalidrawApi,
    ]);
    
    //--
    return (
        <ExcalidrawContext.Provider value={excalidrawApi}>
            <SketchEditorErrorBoundary>
                <Excalidraw
                    excalidrawAPI={api => excalidrawApi.current = api}
                    initialData={editor?.getSceneData()}
                    name={editor?.resource?.name}
                    theme={activeTheme}
                    detectScroll={true}
                    validateEmbeddable={true}
                    handleKeyboardGlobally={true}
                    gridModeEnabled={gridModeEnabled}
                    viewModeEnabled={viewModeEnabled}
                    zenModeEnabled={zenModeEnabled}
                    UIOptions={{
                        welcomeScreen: false,
                        tools: { image: true },
                        // dockedSidebarBreakpoint: 800,
                        canvasActions: {
                            loadScene: false,
                            saveAsImage: false,
                            saveToActiveFile: false,
                            toggleTheme: true,
                            export: false,
                        },
                    }}
                    onChange={async (elements, appState, files) => {
                        // Note: All local state mutations should be guarded,
                        //  unless we like recursion errors for some reason.
                        
                        // TODO: Batch update everywhere possible!
                        
                        if (selectedElementIds != appState.selectedElementIds) {
                            setSelectedElementIds(appState.selectedElementIds);
                        }
                        
                        if (viewModeEnabled != appState.viewModeEnabled) {
                            setViewModeEnabled(appState.viewModeEnabled);
                        }
                        
                        if (zenModeEnabled != appState.zenModeEnabled) {
                            setZenModeEnabled(appState.zenModeEnabled);
                        }
                        
                        if (gridModeEnabled != appState.gridModeEnabled) {
                            setGridModeEnabled(appState.gridModeEnabled);
                        }
                        
                        if (sidebarDocked != appState.defaultSidebarDockedPreference) {
                            setSidebarDocked(appState.defaultSidebarDockedPreference);
                        }
                        
                        if (openSidebar != appState.openSidebar) {
                            setOpenSidebar(appState.openSidebar ?? undefined);
                        }
                        
                        if (activeTool != appState.activeTool) {
                            setActiveTool(appState.activeTool);
                        }
                        
                        // TODO: Can we shorted this to just compare the embeddable itself?
                        if (activeEmbeddable?.element.id != appState.activeEmbeddable?.element.id) {
                            setActiveEmbeddable(appState.activeEmbeddable);
                        }
                        
                        if (selectedGroupIds != appState.selectedGroupIds) {
                            setSelectedGroupIds(appState.selectedGroupIds);
                        }
                        
                        if (selectedElementIds != appState.selectedElementIds) {
                            setSelectedElementIds(appState.selectedElementIds);
                        }
                        
                        // TODO: Store files somewhere, too!
                        if (editor) {
                            editor.updateSceneData({ elements, appState });
                        }
                    }}
                    onScrollChange={(scrollX, scrollY, zoom) => {
                        if (excalidrawApi && lockScrollPos && !isAnimating) {
                            excalidrawApi.current?.updateScene({
                                appState: {
                                    // TODO: Account for offset after zoom.
                                    scrollX: lockScrollPos[0],
                                    scrollY: lockScrollPos[1],
                                    zoom: zoom,
                                },
                                captureUpdate: CaptureUpdateAction.IMMEDIATELY,
                            });
                        }
                    }}
                    onLinkOpen={(element, event) => {
                        event.preventDefault();
                        console.log(`Open Link:`, element, event);
                    }}
                    onPaste={(data, event): boolean => {
                        console.debug(`Pasted:`, data);
                        return true;
                    }}
                    onDuplicate={() => {}}
                    onLibraryChange={() => {}}
                    onPointerUpdate={() => {}}
                    onPointerDown={() => {}}
                    onPointerUp={() => {}}
                    onUserFollow={() => {}}
                    generateIdForFile={async (file) => {
                        console.debug(`TODO: Generate ID for:`, file);
                        return "[resource-id]"; // TODO
                    }}
                    generateLinkForSelection={(id, type) => {
                        console.debug(`TODO: Generate link!`);
                        return "[selection-link]"; // TODO
                    }}
                    renderTopRightUI={(isMobile, appState) => {
                        return (
                            // Render to the top-right corner of the outer
                            // excalidraw element. Can escape bounds with
                            // `position: absolute`.
                            null // TODO
                        )
                    }}
                    renderCustomStats={props => {
                        return (
                            <div>TODO</div>
                        )
                    }}
                    renderEmbeddable={(element, appState) => {
                        return (
                            <ResourceEmbed
                                // link={element.link ?? undefined}
                                element={element}
                            />
                        )
                    }}
                >
                    <MainMenu
                        onSelect={(selectEvent) => {
                            console.debug(`Selected:`, selectEvent);
                        }}
                    >
                        <MainMenu.Group title="Resource">
                            <MainMenu.DefaultItems.Export />
                            <MainMenu.Item
                                onSelect={async event => {
                                    //..
                                }}
                            >
                                Get Rekt
                            </MainMenu.Item>
                        </MainMenu.Group>
                        <MainMenu.Group title="Collaborators">
                            <MainMenu.Item onSelect={() => window.alert("Item1")}>
                                Lorren
                                <MainMenu.Item.Badge type="blue">
                                    You
                                </MainMenu.Item.Badge>
                            </MainMenu.Item>
                            <MainMenu.Item onSelect={() => window.alert("Item2")}>
                                Allie
                                <MainMenu.Item.Badge type="green">
                                    c
                                </MainMenu.Item.Badge>
                            </MainMenu.Item>
                            <MainMenu.Item onSelect={() => window.alert("Item2")}>
                                Jamie
                                <MainMenu.Item.Badge type="red">
                                    d
                                </MainMenu.Item.Badge>
                            </MainMenu.Item>
                        </MainMenu.Group>
                        <MainMenu.Group title="Scene">
                            <MainMenu.DefaultItems.ToggleTheme />
                            <MainMenu.DefaultItems.ChangeCanvasBackground />
                            <MainMenu.ItemCustom>
                                Better Background Selector
                            </MainMenu.ItemCustom>
                        </MainMenu.Group>
                    </MainMenu>
                    <Sidebar
                        name="editor"
                        className="editor-sidebar"
                        docked={sidebarDocked}
                        onDock={isDocked => {
                            excalidrawApi.current?.updateScene({
                                appState: {
                                    defaultSidebarDockedPreference: isDocked,
                                }
                            });
                        }}
                        onStateChange={sidebarState => {
                            // console.debug("Editor Sidebar State Changed:", sidebarState);
                        }}
                    >
                        <Sidebar.Header>
                            <SceneOverviewPanel
                                activeEmbeddable={activeEmbeddable ?? undefined}
                                selectedGroupIds={selectedGroupIds}
                                selectedElementIds={selectedElementIds}
                            />
                        </Sidebar.Header>
                        <Sidebar.Tabs>
                            <Sidebar.Tab tab="controls">
                                <ElementControlPanel
                                    activeEmbeddable={activeEmbeddable ?? undefined}
                                    selectedGroupIds={selectedGroupIds}
                                    selectedElementIds={selectedElementIds}
                                />
                            </Sidebar.Tab>
                            <Sidebar.Tab tab="styles">
                                <section style={{ margin: "0 1rem" }}>
                                    Styles
                                </section>
                            </Sidebar.Tab>
                            <Sidebar.Tab tab="inspect">
                                <ElementInspectorPanel
                                    activeEmbeddable={activeEmbeddable ?? undefined}
                                    selectedGroups={selectedGroupIds}
                                    selectedElements={selectedElementIds}
                                />
                            </Sidebar.Tab>
                            <footer>
                                <nav style={{ margin: "0 1rem" }}>
                                    <Sidebar.TabTriggers>
                                        <Sidebar.TabTrigger tab="controls">Controls</Sidebar.TabTrigger>
                                        <Sidebar.TabTrigger tab="styles">Styles</Sidebar.TabTrigger>
                                        <Sidebar.TabTrigger tab="inspect">Inspect</Sidebar.TabTrigger>
                                    </Sidebar.TabTriggers>
                                </nav>
                            </footer>
                        </Sidebar.Tabs>
                    </Sidebar>
                    <Sidebar
                        name="settings"
                        docked={sidebarDocked}
                        onDock={setSidebarDocked}
                        onStateChange={tabState => {
                            console.debug("Settings Sidebar State Changed:", tabState);
                        }}
                    >
                        <Sidebar.Header>
                            TODO
                        </Sidebar.Header>
                        <Sidebar.Tabs>
                            <Sidebar.Tab tab="preferences">
                                <section className="scene-controls" style={{ margin: "0 1rem" }}>
                                    <fieldset>
                                        <legend>Canvas</legend>
                                        <label>
                                            <input type="checkbox" 
                                                checked={viewModeEnabled}
                                                onChange={({ target }) => {
                                                    setViewModeEnabled(target.checked);
                                                }}
                                            />
                                            <span>View mode</span>
                                        </label>
                                        <label>
                                            <input type="checkbox"
                                                checked={zenModeEnabled}
                                                onChange={({ target }) => {
                                                    setZenModeEnabled(target.checked);
                                                }}
                                            />
                                            <span>Zen mode</span>
                                        </label>
                                        <label>
                                            <input type="checkbox"
                                                checked={gridModeEnabled}
                                                onChange={({ target }) => {
                                                    setGridModeEnabled(target.checked);
                                                }}
                                            />
                                            <span>Enable Grid Mode</span>
                                        </label>
                                    </fieldset>
                                </section>
                                <section className="export-controls" style={{ margin: "0 1rem" }}>
                                    <fieldset>
                                        <legend>Export</legend>
                                        <label>
                                            <input type="checkbox"
                                                checked={false}
                                                onChange={() => {
                                                    console.debug(`TODO`);
                                                }}
                                            />
                                            <span>Export with dark mode</span>
                                        </label>
                                        <label>
                                            <input type="checkbox"
                                                checked={false}
                                                onChange={() => {
                                                    console.debug(`TODO`)
                                                }}
                                            />
                                            <span>Export with embed scene</span>
                                        </label>
                                    </fieldset>
                                </section>
                            </Sidebar.Tab>
                            <Sidebar.Tab tab="co-op">
                                <section>
                                    TODO
                                </section>
                            </Sidebar.Tab>
                            <nav style={{ margin: "0 1rem" }}>
                                <Sidebar.TabTriggers>
                                    <Sidebar.TabTrigger tab="preferences">Prefs</Sidebar.TabTrigger>
                                    <Sidebar.TabTrigger tab="co-op">Co-op</Sidebar.TabTrigger>
                                </Sidebar.TabTriggers>
                            </nav>
                        </Sidebar.Tabs>
                    </Sidebar>
                    <EditorToolsOverlay
                        // TODO
                        gridModeEnabled={gridModeEnabled}
                        viewModeEnabled={viewModeEnabled}
                        zenModeEnabled={zenModeEnabled}
                        // lockScrollPos={[lockScrollPos, setLockScrollPos]}
                    />
                    <ResourceDisplayOverlay
                        resource={editor?.resource}
                        debugMode={!zenModeEnabled}
                    />
                    <Footer>
                        <StatusDisplayToolbar />
                        <SceneActionsToolbar />
                    </Footer>
                </Excalidraw>
            </SketchEditorErrorBoundary>
        </ExcalidrawContext.Provider>
    )
}

export function StatusDisplayToolbar() {
    return (
        <section className="status">
            TODO: Status(es)
        </section>
    )
}

export interface SceneActionsToolbarProps {
    //..
}

export function SceneActionsToolbar(props: SceneActionsToolbarProps) {
    return (
        <section className="actions">
            <button className="run">
                <svg xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    className="icon play lucide"
                >
                    <path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z" />
                </svg>
                <span>Run</span>
            </button>
            <button className="build">
                <svg xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    className="icon hammer lucide"
                >
                    <path d="m15 12-9.373 9.373a1 1 0 0 1-3.001-3L12 9"/>
                    <path d="m18 15 4-4"/>
                    <path d="m21.5 11.5-1.914-1.914A2 2 0 0 1 19 8.172v-.344a2 2 0 0 0-.586-1.414l-1.657-1.657A6 6 0 0 0 12.516 3H9l1.243 1.243A6 6 0 0 1 12 8.485V10l2 2h1.172a2 2 0 0 1 1.414.586L18.5 14.5"/>
                </svg>
                <span>Build</span>
            </button>
            <button className="debug">
                <svg xmlns="http://www.w3.org/2000/svg"
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    className="lucide lucide-bug-play-icon lucide-bug-play"
                >
                    <path d="M10 19.655A6 6 0 0 1 6 14v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 3.97"/>
                    <path d="M14 15.003a1 1 0 0 1 1.517-.859l4.997 2.997a1 1 0 0 1 0 1.718l-4.997 2.997a1 1 0 0 1-1.517-.86z"/>
                    <path d="M14.12 3.88 16 2"/>
                    <path d="M21 5a4 4 0 0 1-3.55 3.97"/>
                    <path d="M3 21a4 4 0 0 1 3.81-4"/>
                    <path d="M3 5a4 4 0 0 0 3.55 3.97"/>
                    <path d="M6 13H2"/>
                    <path d="m8 2 1.88 1.88"/>
                    <path d="M9 7.13V6a3 3 0 1 1 6 0v1.13"/>
                </svg>
                <span>Debug</span>
            </button>
        </section>
    )
}