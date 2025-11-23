// deno-lint-ignore-file
import React, { useState, useEffect } from "react";

import { Excalidraw, Footer, Sidebar, useHandleLibrary } from "@excalidraw/excalidraw";
import { MIME_TYPES, loadSceneOrLibraryFromBlob } from "@excalidraw/excalidraw";
import type { ImportedLibraryData } from "@excalidraw/excalidraw/data/types";
import type { ExcalidrawEmbeddableElement, NonDeleted, NonDeletedExcalidrawElement, Theme } from "@excalidraw/excalidraw/element/types";
import type { AppState, ExcalidrawImperativeAPI, ExcalidrawInitialDataState, UIAppState } from "@excalidraw/excalidraw/types";


import { fileOpen, fromLocalStorage } from "../../utils.ts";

import "../../app.css";

// TODO: Move this out to a declaration file.
declare global {
  // TODO: Declare EXCALIDRAW_ASSET_PATH on Window or globalThis??
  var EXCALIDRAW_ASSET_PATH: string;
}

// TODO: Can we set this per-instance?
globalThis.EXCALIDRAW_ASSET_PATH = "https://cdn.jsdelivr.net/npm/@excalidraw/excalidraw/dist/prod/";

export interface SketchWorkspaceProps {
  displayName: string;
  initialData?: ExcalidrawInitialDataState,
}

export default function Workspace(props: SketchWorkspaceProps) {
  const { displayName, initialData } = props;

  const [excalidrawAPI, setExcalidrawAPI] = useState<ExcalidrawImperativeAPI | null>(null);

  const [theme, setTheme] = useState<Theme>("dark");

  const [gridModeEnabled, setGridModeEnabled] = useState(true);
  const [viewModeEnabled, setViewModeEnabled] = useState(false);
  const [zenModeEnabled, setZenModeEnabled] = useState(false);

  const [renderScrollbars, setRenderScrollbars] = useState(false);
  const [disableImageTool, setDisableImageTool] = useState(false);

  const [canvasUrl, setCanvasUrl] = useState<string | undefined>(undefined);

  const [exportWithDarkMode, setExportWithDarkMode] = useState(false);
  const [exportEmbedScene, setExportEmbedScene] = useState(false);

  const [sidebarDocked, setSidebarDocked] = useState<boolean>(fromLocalStorage("excalidraw:is-sidebar-docked"));

  // const initialStatePromiseRef = useRef<SketchWorkspaceStateRef>({ excalidraw: null! }); // Removed

  // New simplified state calculation for initial scene data
  const [initialSceneData] = useState<ExcalidrawInitialDataState | null>(() => {
    const defaultSceneState: ExcalidrawInitialDataState = {
      appState: {
        currentItemStrokeColor: "transparent",
        currentItemBackgroundColor: "#e9ecef",
        currentItemStrokeWidth: 1,
        currentItemRoughness: 0,
        currentItemRoundness: "sharp",
        defaultSidebarDockedPreference: sidebarDocked,
      }
    };

    let savedData = null;
    try {
      const storedScene = localStorage.getItem(`excalidraw:saved-scene-state`);
      if (storedScene) {
        savedData = JSON.parse(storedScene);
      }
    } catch (error) {
      console.error("Failed to load scene from localStorage", error);
    }

    // Merge: defaults < initialData prop < savedData
    return Object.assign(defaultSceneState, initialData, savedData);
  });

  // useHandleLibrary({ excalidrawAPI });

  useEffect(() => {
    try {
      localStorage.setItem("excalidraw:is-sidebar-docked", JSON.stringify(sidebarDocked));
    } catch (e) {
      console.error("Failed to save sidebar state to localStorage", e);
    }
  }, [sidebarDocked]);

  const loadSceneOrLibrary = async () => {
    const file = await fileOpen({ description: "Excalidraw or library file" });
    const contents = await loadSceneOrLibraryFromBlob(file, null, null);
    if (contents.type === MIME_TYPES.excalidraw) {
      const sceneData = contents.data;
      if (sceneData.appState) {
        // TODO: Allow the user to set a preferred theme in settings.
        sceneData.appState.theme = theme;
      }
      excalidrawAPI?.updateScene(sceneData);
    } else if (contents.type === MIME_TYPES.excalidrawlib) {
      excalidrawAPI?.updateLibrary({
        libraryItems: (contents.data as ImportedLibraryData).libraryItems!,
        openLibraryMenu: true,
      });
    }
  };

  return (
    <Excalidraw
      excalidrawAPI={(api) => setExcalidrawAPI(api)}
      initialData={initialSceneData}
      name={displayName}
      theme={theme}
      viewModeEnabled={viewModeEnabled}
      zenModeEnabled={zenModeEnabled}
      gridModeEnabled={gridModeEnabled}
      validateEmbeddable={true}
      autoFocus={true}
      UIOptions={{
        canvasActions: {
          loadScene: true,
          saveToActiveFile: true,
          export: {
            saveFileToDisk: true,
            onExportToBackend: () => {
              //..
            },
            renderCustomUI: (
              exportedElements: readonly NonDeletedExcalidrawElement[],
              appState: any,
              files: any,
              canvas: HTMLCanvasElement
            ) => {
              // You can customize this UI as needed
              return (
                <div>
                  <span>Custom export UI placeholder</span>
                </div>
              );
            },
          },
          toggleTheme: false,
        },
        tools: {
          image: !disableImageTool,
        },
      }}
      renderTopRightUI={(
        isMobile: boolean,
        appState: UIAppState,
      ) => {
        // You can customize this UI as needed, or return null for now
        return null;
      }}
      renderEmbeddable={(
        element: NonDeleted<ExcalidrawEmbeddableElement>,
        appState: AppState,
      ) => {
        try {
          // Internal paths.
          if (element.link?.match(/^\//i)) {
            return (
              <iframe src={element.link} style={{ width: "100%", height: "100%", border: "none" }}>
                Embeded URL
              </iframe>
            )
          }
          // External URLs.
          if (element.link?.match(/^(http)/i)) {
            // TODO: Detect when an iframe embed might fail (an http page
            // embedding an https), for example) and let the user know why
            // the embed isn't showing the page's contents.
            const embededReso0urce = new URL(element.link);
            if (embededReso0urce.host) {
              // TODO: Detect errors loading the embedded page and attempt
              // to show error output for the user.
              return (
                <iframe src={element.link} style={{ width: "100%", height: "100%", border: "none" }}>
                  Embeded URL
                </iframe>
              )
            }
          }
          // Special case to inspect the block.
          if (element.link === "note") {
            const onKey = (event: any) => event?.stopPropagation();
            const onChange = (event: any) => {
              // ..
            };
            return (
              <div
                contentEditable={true}
                style={{ width: "100%", height: "100%", border: "none" }}
                onChange={onChange}
                onKeyDown={onKey}
                onKeyUp={onKey}
              >
                TODO
              </div>
            )
          }
        } catch (error) {
          console.warn(`Couldn't parse URL '${element.link}':`, error);
        }

        return null;
      }}
      onChange={(
        elements: readonly NonDeletedExcalidrawElement[],
        appState: AppState,
        files: any
      ) => {
        // console.debug(`State Changed!`);
        // console.debug("Elements :", elements, "State : ", appState, "Files: ", files);
      }}
      onLinkOpen={(
        element: NonDeletedExcalidrawElement,
        event: CustomEvent<{
          nativeEvent: MouseEvent | React.PointerEvent<HTMLCanvasElement>;
        }>,
      ) => {
        const link = element.link!;
        const isNewTab = event.detail.nativeEvent.ctrlKey || event.detail.nativeEvent.metaKey;
        const isNewWindow = event.detail.nativeEvent.shiftKey;
        const isInternalLink = link.startsWith("/") || link.includes(window.location.origin);

        if (isInternalLink && !isNewTab && !isNewWindow) {
          // signal that we're handling the redirect ourselves
          event.preventDefault();
          // do a custom redirect, such as passing to react-router
          // ...
        }
      }}
    >
      <Sidebar
        name="editor"
        docked={sidebarDocked}
        onDock={sidebarDocked => {
          setSidebarDocked(!sidebarDocked);
        }}
        onStateChange={(state: any) => {
          console.debug("Sidebar State Changed:", state)
        }}
      >
        <Sidebar.Header>
          Editor
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
            <section style={{ margin: "0 1rem" }}>
              Inspect
            </section>
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
            <section style={{ margin: "0 1rem" }}>
              Preferences
            </section>
          </Sidebar.Tab>
          <Sidebar.Tab tab="co-op">
            <section style={{ margin: "0 1rem" }}>
              <div className="scene-controls">
                <button onClick={loadSceneOrLibrary}>
                  Load Scene or Library
                </button>
                <button className="reset-scene" onClick={() => excalidrawAPI?.resetScene()}>
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
                  <input type="checkbox" checked={disableImageTool === true} onChange={() => { setDisableImageTool(!disableImageTool); }} />
                  Disable Image Tool
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
                          
                          excalidrawAPI?.updateScene({ collaborators });
                        } else {
                          excalidrawAPI?.updateScene({ collaborators: new Map() });
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
                    if (!excalidrawAPI) {
                      return;
                    }
                    const elements = excalidrawAPI.getSceneElements();
                    excalidrawAPI.scrollToContent(elements[0], {
                      fitToViewport: true,
                    });
                  }
                }>
                  Fit to viewport, first element
                </button>
                <button type="button" onClick={
                  event => {
                    if (!excalidrawAPI) {
                      return;
                    }
                    const elements = excalidrawAPI.getSceneElements();
                    excalidrawAPI.scrollToContent(elements[0], {
                      fitToContent: true,
                    });
                    excalidrawAPI.scrollToContent(elements[0], {
                      fitToContent: true,
                    });
                  }
                }>
                  Fit to content, first element
                </button>
                <button type="button" onClick={
                  event => {
                    if (!excalidrawAPI) {
                      return;
                    }

                    const elements = excalidrawAPI.getSceneElements();
                    excalidrawAPI.scrollToContent(elements[0], {
                      fitToContent: true,
                    });

                    excalidrawAPI.scrollToContent(elements[0]);
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