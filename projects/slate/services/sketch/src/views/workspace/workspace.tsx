// deno-lint-ignore-file

// TODO: As of Nov. 15 2025, `deno bundle --platform web` seems
//   to fail when using @std/path. Maybe misconfigured?
// import { join } from "@std/path";

import React, { useState, useEffect } from "react";

import { CaptureUpdateAction, Excalidraw, Footer, Sidebar, useHandleLibrary } from "@excalidraw/excalidraw";
import { MIME_TYPES, loadSceneOrLibraryFromBlob } from "@excalidraw/excalidraw";
import type { ImportedLibraryData } from "@excalidraw/excalidraw/data/types";
import type { ExcalidrawEmbeddableElement, NonDeleted, NonDeletedExcalidrawElement, Theme } from "@excalidraw/excalidraw/element/types";
import type { AppState, BinaryFiles, ExcalidrawImperativeAPI, ExcalidrawInitialDataState, UIAppState } from "@excalidraw/excalidraw/types";

import { fileOpen, fromLocalStorage } from "../../utils.ts";

import "../../app.css";

// TODO: Move this to a decl. file in the types dir.
declare global {
  var EXCALIDRAW_ASSET_PATH: string;
  var SKETCH_ASSET_PATH: string | undefined;
}

// TODO: Can we set this per-instance?
// globalThis.EXCALIDRAW_ASSET_PATH = "https://cdn.jsdelivr.net/npm/@excalidraw/excalidraw/dist/prod/";

export async function loadSceneOrLibrary(excalidraw: ExcalidrawImperativeAPI) {
  const file = await fileOpen({ description: "Excalidraw or library file" });
  const contents = await loadSceneOrLibraryFromBlob(file, null, null);
  if (contents.type === MIME_TYPES.excalidraw) {
    const sceneData = contents.data;
    if (sceneData.appState) {
      // TODO: Allow the user to set a preferred theme in settings.
      const currentAppState = excalidraw.getAppState();
      sceneData.appState.theme = currentAppState.theme;
    }
    excalidraw.updateScene(sceneData);
  } else if (contents.type === MIME_TYPES.excalidrawlib) {
    excalidraw.updateLibrary({
      libraryItems: (contents.data as ImportedLibraryData).libraryItems!,
      openLibraryMenu: true,
    });
  }
};

export interface SketchEditorProps {
  $store?: any;
  displayName?: string;
  loadSketch?: boolean;
  saveSketch?: boolean;
  svgData?: ExcalidrawInitialDataState,
  onOpenResource: (element: { link: any }) => void,
  onSceneReady: () => void,
  onChange: (elements: readonly NonDeletedExcalidrawElement[], appState: AppState, files: any) => void,
}

export default function SketchEditor(props: SketchEditorProps) {
  const [canvasUrl, setCanvasUrl] = useState(undefined);

  //..
  const [theme, setTheme] = useState<Theme>("dark");

  const [gridModeEnabled, setGridModeEnabled] = useState(true);
  const [viewModeEnabled, setViewModeEnabled] = useState(false);
  const [zenModeEnabled, setZenModeEnabled] = useState(false);
  const [renderScrollbars, setRenderScrollbars] = useState(false);

  const [exportWithDarkMode, setExportWithDarkMode] = useState(true);
  const [exportEmbedScene, setExportEmbedScene] = useState(true);

  const [toolsEnabled, setToolsEnabled] = useState({ image: true });

  useEffect(() => {
    console.debug(`TODO: `);
  }, [
    gridModeEnabled,
    viewModeEnabled,
    zenModeEnabled,
    renderScrollbars,
  ]);

  //..
  const [sidebarDocked, setSidebarDocked] = useState(fromLocalStorage("excalidraw:is-sidebar-docked") ?? false);
  // const commitSidebarDocked = () => {
  //   try {
  //     // Save to the state manager/context ..
  //     localStorage.setItem("excalidraw:is-sidebar-docked", JSON.stringify(sidebarDocked));
  //   } catch (e) {
  //     console.error("Failed to save sidebar state to localStorage", e);
  //   }
  // };
  // useEffect(commitSidebarDocked, [sidebarDocked]);

  //..
  const [excalidrawBackend, setExcalidrawBackend] = useState<ExcalidrawImperativeAPI | null>(null);

  const elements = excalidrawBackend?.getSceneElements();
  const sceneState = excalidrawBackend?.getAppState();

  // TODO: Move this to a managed backend/wrapper around excalidraw's api.
  const [sceneData, setSceneData] = useState(props.svgData);

  const [activeTool, setActiveTool] = useState(sceneState?.activeTool);
  const [cursorButton, setCursorButton] = useState(sceneState?.cursorButton);
  const [activeEmbeddable, setActiveEmbeddable] = useState(sceneState?.activeEmbeddable);
  const [selectedElementIds, setSelectedElementIds] = useState(sceneState?.selectedElementIds);
  const [previousSelectedElementIds, setPreviousSelectedElementIds] = useState(sceneState?.previousSelectedElementIds);
  const [showHyperlinkPopup, setShowHyperlinkPopup] = useState(sceneState?.showHyperlinkPopup);

  useEffect(() => {
    try {
      // Save to the state manager/context ..
      localStorage.setItem("excalidraw:saved-scene-state", JSON.stringify({ elements, sceneData }));
    } catch (e) {
      console.error("Failed to save sidebar state to localStorage", e);
    }
  }, [elements, sceneData]);

  return (
    <Excalidraw
      excalidrawAPI={setExcalidrawBackend}
      initialData={sceneData}
      name={props.displayName}
      theme={theme}
      viewModeEnabled={viewModeEnabled}
      zenModeEnabled={zenModeEnabled}
      gridModeEnabled={gridModeEnabled}
      validateEmbeddable={true}
      autoFocus={true}
      UIOptions={{
        welcomeScreen: true,
        dockedSidebarBreakpoint: 100,
        tools: toolsEnabled,
        canvasActions: {
          loadScene: props.loadSketch ?? false,
          saveAsImage: props.saveSketch ?? false,
          saveToActiveFile: props.saveSketch ?? false,
          toggleTheme: true,
          export: false,
          // export: {
          //   saveFileToDisk: false,
          //   onExportToBackend: () => {
          //     console.debug(`Called onExportToBackend(..):`, arguments);
          //   },
          //   renderCustomUI: (
          //     exportedElements: readonly NonDeletedExcalidrawElement[],
          //     appState: UIAppState,
          //     files: BinaryFiles,
          //     canvas: HTMLCanvasElement
          //   ) => {
          //     // You can customize this UI as needed
          //     return (
          //       <div>
          //         <span>TODO: Do we need a custom export?</span>
          //       </div>
          //     );
          //   },
          // },
        },
      }}
      onScrollChange={(scrollX, scrollY, zoom) => {
        // const BOUND = 3000;

        // console.log(`Got scroll change!`);
        // let newScrollX = scrollX;
        // let newScrollY = scrollY;

        // // Clamp the scrollX position
        // if (scrollX > BOUND) {
        //   newScrollX = BOUND;
        // } else if (scrollX < -BOUND) {
        //   newScrollX = -BOUND;
        // }

        // // Clamp the scrollY position
        // if (scrollY > BOUND) {
        //   newScrollY = BOUND;
        // } else if (scrollY < -BOUND) {
        //   newScrollY = -BOUND;
        // }

        // // If a change was necessary, force an update via the API
        // if (newScrollX !== scrollX || newScrollY !== scrollY) {
        //   excalidrawBackend?.updateScene({
        //     appState: {
        //       scrollX: newScrollX,
        //       scrollY: newScrollY,
        //     },
        //     captureUpdate: "IMMEDIATELY",
        //   });
        // }
      }}
      onLinkOpen={(
        element: NonDeletedExcalidrawElement,
        event: CustomEvent<{
          nativeEvent: MouseEvent | React.PointerEvent<HTMLCanvasElement>;
        }>,
      ) => {
        event.preventDefault();

        const link = element.link!;
        const isNewTab = event.detail.nativeEvent.ctrlKey || event.detail.nativeEvent.metaKey;
        const isNewWindow = event.detail.nativeEvent.shiftKey;
        const isInternalLink = link.startsWith("/") || link.includes(window.location.origin);

        if (isInternalLink && !isNewTab && !isNewWindow) {
          // signal that we're handling the redirect ourselves
          // do a custom redirect, such as passing to react-router
          // ...
        }

        props.onOpenResource(element);
      }}
      onChange={(elements, appState, files) => {
        setActiveTool(appState.activeTool);
        setActiveEmbeddable(appState.activeEmbeddable);
        setSelectedElementIds(appState.selectedElementIds);
        setCursorButton(appState.cursorButton);

        props.onChange(elements, appState, files);
      }}
      renderTopRightUI={(isMobile: boolean, appState: UIAppState) => {
        return (
          <div style={{ display: "flex", flexDirection: "row", alignItems: "center" }}>
            TODO: Run/Build/Debug
          </div>
        )
      }}
      renderEmbeddable={(
        element: NonDeleted<ExcalidrawEmbeddableElement>,
        appState: AppState,
      ) => {
        const embedContainer = {
          width: "100%",
          height: "100%",
          border: "none",
        };
        try {
          // Relative to the current workspace dev service root path.
          if (element.link?.startsWith("/")) {
            return (
              <iframe src={element.link} style={{ ...embedContainer }}>
                Embeded URL
              </iframe>
            )
          }
          // Relative to the current workspace local root file directory.
          else if (element.link?.startsWith("./")) {
            const relPrefix = globalThis.SKETCH_ASSET_PATH ?? "";
            const resourcePath = `${relPrefix}${element.link}`;
            return (
              <iframe src={resourcePath} style={{ ...embedContainer }}>
                Embeded URL
              </iframe>
            )
          }
          // External URLs.
          else if (element.link?.startsWith("http")) {
            // TODO: Detect when an iframe embed might fail (an http page
            // embedding an https), for example) and let the user know why
            // the embed isn't showing the page's contents.
            const embededResource = new URL(element.link);
            if (embededResource.host) {
              // TODO: Detect errors loading the embedded page and attempt
              // to show error output for the user.
              return (
                <iframe src={element.link} style={{ ...embedContainer }}>
                  Embeded URL
                </iframe>
              )
            }
          }
          // Special case to inspect the block.
          else if (element.link?.startsWith("#")) {
            const onKey = (event: any) => event?.stopPropagation();
            const onChange = (event: any) => {
              // ..
            };
            return (
              <div
                // contentEditable={true}
                style={{ ...embedContainer }}
                onChange={onChange}
                onKeyDown={onKey}
                onKeyUp={onKey}
              >
                TODO
              </div>
            )
          }
          // Everything else.
          else {
            console.debug(`TODO: Fallback rendering?`);
          }
        } catch (error) {
          console.warn(`Couldn't parse URL '${element.link}':`, error);
        }

        return null;
      }}
    >
      <Sidebar
        name="editor"
        docked={sidebarDocked}
        onDock={sidebarDocked => {
          console.debug(`Sidebar Docked?`, sidebarDocked);
          setSidebarDocked(!sidebarDocked);
        }}
        onStateChange={(state: any) => {
          console.debug("Sidebar State Changed:", state)
        }}
      >
        <Sidebar.Header>
          <section>
            Editor
          </section>
        </Sidebar.Header>
        <Sidebar.Tabs>
          <Sidebar.Tab tab="controls">
            <section style={{ margin: "0 1rem" }}>
              Controls
            </section>
          </Sidebar.Tab>
          <Sidebar.Tab tab="styles">
            <section style={{ margin: "0 1rem" }}>
              Styles
            </section>
          </Sidebar.Tab>
          <Sidebar.Tab tab="inspect">
            <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
              <section style={{ margin: "0 1rem", flexGrow: 0 }}>
                Inspect
              </section>
              <div style={{ overflowY: "auto", flexGrow: 1 }}>
                <section>
                  <ul>
                    <li>Excalidraw ID: {excalidrawBackend?.id}</li>
                  </ul>
                </section>
                <section style={{ margin: "0 1rem" }}>
                  <h4>Active Tool</h4>
                  <pre>
                    <code>
                      {JSON.stringify(activeTool, null, 2)}
                    </code>
                  </pre>
                </section>
                <section style={{ margin: "0 1rem" }}>
                  <h4>Active Embeddable</h4>
                  <pre>
                    <code>
                      {JSON.stringify(activeEmbeddable, null, 2)}
                    </code>
                  </pre>
                </section>
                <section style={{ margin: "0 1rem" }}>
                  <h4>Selected Elements</h4>
                  <pre>
                    <code>
                      {
                        // JSON.stringify(selectedElementIds, null, 2);
                        selectedElementIds?.length && Object.entries(selectedElementIds).map(([id, _], i) => {
                          // TODO: Pull these in batch above ..
                          const sceneElements = excalidrawBackend?.getSceneElements();
                          const selectedElement = sceneElements?.find(element => element.id == id);
                          if (selectedElement) {
                            return (
                              <div>
                                Element #{id}
                              </div>
                            )
                          } else {
                            return (
                              <div>
                                Element #{id} not found ..
                              </div>
                            )
                          }
                        })
                      }
                    </code>
                  </pre>
                </section>
              </div>
            </div>
          </Sidebar.Tab>
          <nav style={{ margin: "0 1rem" }}>
            <Sidebar.TabTriggers>
              <Sidebar.TabTrigger tab="controls">Controls</Sidebar.TabTrigger>
              <Sidebar.TabTrigger tab="styles">Styles</Sidebar.TabTrigger>
              <Sidebar.TabTrigger tab="inspect">Inspect</Sidebar.TabTrigger>
            </Sidebar.TabTriggers>
          </nav>
        </Sidebar.Tabs>
      </Sidebar>
      <Sidebar
        name="settings"
        docked={sidebarDocked}
        onDock={sidebarDocked => setSidebarDocked(!sidebarDocked)}
        onStateChange={state => console.debug("Sidebar State Changed:", state)}
      >
        <Sidebar.Header>
          Settings
        </Sidebar.Header>
        <Sidebar.Tabs>
          <Sidebar.Tab tab="preferences">
            <header style={{ margin: "0 1rem" }}>
              Preferences
            </header>
            <section style={{ margin: "0 1rem" }}>
              <div className="scene-controls">
                {/* TODO: Ensure backed is available (above).*/}
                <button onClick={_ => loadSceneOrLibrary(excalidrawBackend!)}>
                  Load Scene or Library
                </button>
                <button className="reset-scene" onClick={() => excalidrawBackend?.resetScene()}>
                  Reset Scene
                </button>
                <fieldset>
                  <label>
                    <input type="checkbox" checked={viewModeEnabled} onChange={() => setViewModeEnabled(!viewModeEnabled)} />
                    <span className="label">View mode</span>
                  </label>
                </fieldset>
                <fieldset>
                  <label>
                    <input type="checkbox" checked={zenModeEnabled} onChange={() => setZenModeEnabled(!zenModeEnabled)} />
                    <span className="label">Zen mode</span>
                  </label>
                </fieldset>
                <fieldset>
                  <label>
                    <input type="checkbox" checked={gridModeEnabled} onChange={() => setGridModeEnabled(!gridModeEnabled)} />
                    <span className="label">Grid mode</span>
                  </label>
                </fieldset>
                <label>
                  <input type="checkbox" checked={renderScrollbars} onChange={() => setRenderScrollbars(!renderScrollbars)} />
                  Render scrollbars
                </label>
                <label>
                  <input type="checkbox"
                    checked={toolsEnabled?.image === true}
                    onChange={() => {
                      setToolsEnabled({ image: !toolsEnabled?.image });
                    }}
                  />
                  <span>Disable Image Tool</span>
                </label>
                {
                  /**
                  TODO: Connected clients list:
                  <label>
                    <input type="checkbox" checked={isCollaborating} onChange={() => {
                        if (!isCollaborating) {
                          const collaborators = new Map();
                          
                          collaborators.set("id1", { username: "Doremon", avatarUrl: "images/doremon.png" });
                          collaborators.set("id2", { username: "Excalibot", avatarUrl: "images/excalibot.png" });
                          collaborators.set("id3", { username: "Pika", avatarUrl: "images/pika.jpeg" });
                          collaborators.set("id4", { username: "fallback", avatarUrl: "https://example.com" });
                          
                          excalidrawBackend?.updateScene({ collaborators });
                        } else {
                          excalidrawBackend?.updateScene({ collaborators: new Map() });
                        }
                        setIsCollaborating(!isCollaborating);
                      }} />
                    Show collaborators
                  </label>
                  */
                }
              </div>
              <div className="export-controls button-wrapper">
                <label>
                  <input type="checkbox" checked={exportWithDarkMode} onChange={() => setExportWithDarkMode(!exportWithDarkMode)} />
                  Export with dark mode
                </label>
                <label>
                  <input type="checkbox" checked={exportEmbedScene} onChange={() => setExportEmbedScene(!exportEmbedScene)} />
                  Export with embed scene
                </label>
                <button type="button" onClick={
                  event => {
                    if (!excalidrawBackend) {
                      return;
                    }
                    const elements = excalidrawBackend.getSceneElements();
                    excalidrawBackend.scrollToContent(elements[0], {
                      fitToViewport: true,
                    });
                  }
                }>
                  Fit to viewport, first element
                </button>
                <button type="button" onClick={
                  event => {
                    if (!excalidrawBackend) {
                      return;
                    }
                    const elements = excalidrawBackend.getSceneElements();
                    excalidrawBackend.scrollToContent(elements[0], {
                      fitToContent: true,
                    });
                    excalidrawBackend.scrollToContent(elements[0], {
                      fitToContent: true,
                    });
                  }
                }>
                  Fit to content, first element
                </button>
                <button type="button" onClick={
                  event => {
                    if (!excalidrawBackend) {
                      return;
                    }

                    const elements = excalidrawBackend.getSceneElements();
                    excalidrawBackend.scrollToContent(elements[0], {
                      fitToContent: true,
                    });

                    excalidrawBackend.scrollToContent(elements[0]);
                  }
                }>
                  Scroll to first element, no fitToContent, no fitToViewport
                </button>
                <div className="export export-canvas">
                  <img src={canvasUrl} alt="" />
                </div>
              </div>
            </section>
          </Sidebar.Tab>
          <Sidebar.Tab tab="co-op">
            <header style={{ margin: "0 1rem" }}>
              Co-op Settings
            </header>
          </Sidebar.Tab>
          <nav style={{ margin: "0 1rem" }}>
            <Sidebar.TabTriggers>
              <Sidebar.TabTrigger tab="preferences">Prefs</Sidebar.TabTrigger>
              <Sidebar.TabTrigger tab="co-op">Co-op</Sidebar.TabTrigger>
            </Sidebar.TabTriggers>
          </nav>
        </Sidebar.Tabs>
      </Sidebar>
      <Footer>
        <div style={{ display: "flex", flex: 1, margin: "0 0.5rem", justifyContent: "right" }}>
          <Sidebar.Trigger name="editor" tab="controls">
            Editor
          </Sidebar.Trigger>
          <Sidebar.Trigger name="settings" tab="preferences">
            Settings
          </Sidebar.Trigger>
        </div>
      </Footer>
    </Excalidraw>
  );
}