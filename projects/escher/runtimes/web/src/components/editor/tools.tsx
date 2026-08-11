import { useContext, useState } from "react";

import { Sidebar } from "@excalidraw/excalidraw";

import { EditorContext, ExcalidrawContext } from "./editor.tsx";
import type { LockScrollPosition, MutableState } from "./editor.tsx";

export interface EditorToolsOverlayProps {
    gridModeEnabled?: boolean,
    viewModeEnabled?: boolean,
    zenModeEnabled?: boolean,
    lockScrollPos?: MutableState<LockScrollPosition>,
    isMobile?: boolean;
}

export function EditorToolsOverlay(props: EditorToolsOverlayProps) {
    const [editor] = useContext(EditorContext);
    const excalidrawApi = useContext(ExcalidrawContext);
    
    // const classNames = props.isMobile
    //     ? "sidebar-tools mobile-misc-tools-container"
    //     : "sidebar-tools misc-tools-container";
    
    return (
        <div
            className="tools-overlay"
            data-view-mode={props.viewModeEnabled}
            data-zen-mode={props.zenModeEnabled}
        >
            <nav className={`sidebar-tools misc-tools-container`}>
                <Sidebar.Trigger
                    name="editor"
                    tab="controls"
                    title="Edit"
                    icon={(
                        <svg xmlns="http://www.w3.org/2000/svg"
                            className="icons pencil-ruler lucide-pencil-ruler"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="#fff"
                        >
                            <path d="M13 7 8.7 2.7a2.41 2.41 0 0 0-3.4 0L2.7 5.3a2.41 2.41 0 0 0 0 3.4L7 13" />
                            <path d="m8 6 2-2" />
                            <path d="m18 16 2-2" />
                            <path d="m17 11 4.3 4.3c.94.94.94 2.46 0 3.4l-2.6 2.6c-.94.94-2.46.94-3.4 0L11 17" />
                            <path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" />
                            <path d="m15 5 4 4" />
                        </svg>
                    )}
                />
                <Sidebar.Trigger
                    name="settings"
                    tab="preferences"
                    title="Settings"
                    icon={(
                        <svg xmlns="http://www.w3.org/2000/svg"
                            className="icons lucide-square-pen"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="#fff"
                        >
                            <path d="M14 17H5" />
                            <path d="M19 7h-9" />
                            <circle cx="17" cy="17" r="3" />
                            <circle cx="7" cy="7" r="3" />
                        </svg>
                    )}
                />
            </nav>
            <div className="canvas-tools">
                <section>
                    <label className="enable-view-mode">
                        <input type="checkbox"
                            checked={props.viewModeEnabled}
                            onChange={event => {
                                excalidrawApi?.current?.updateScene({
                                    appState: {
                                        viewModeEnabled: event.target.checked,
                                    }
                                });
                            }}
                        />
                        <svg xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            className="icon scan-eye lucide"
                        >
                            <path d="M3 7V5a2 2 0 0 1 2-2h2" />
                            <path d="M17 3h2a2 2 0 0 1 2 2v2" />
                            <path d="M21 17v2a2 2 0 0 1-2 2h-2" />
                            <path d="M7 21H5a2 2 0 0 1-2-2v-2" />
                            <circle cx="12" cy="12" r="1" />
                            <path d="M18.944 12.33a1 1 0 0 0 0-.66 7.5 7.5 0 0 0-13.888 0 1 1 0 0 0 0 .66 7.5 7.5 0 0 0 13.888 0" />
                        </svg>
                    </label>
                    <label className="enable-zen-mode">
                        <input type="checkbox"
                            checked={props.zenModeEnabled}
                            onChange={event => {
                                excalidrawApi?.current?.updateScene({
                                    appState: {
                                        zenModeEnabled: event.target.checked,
                                    }
                                });
                            }}
                        />
                        <svg xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            className="icon focus lucide"
                        >
                            <circle cx="12" cy="12" r="3" />
                            <path d="M3 7V5a2 2 0 0 1 2-2h2" />
                            <path d="M17 3h2a2 2 0 0 1 2 2v2" />
                            <path d="M21 17v2a2 2 0 0 1-2 2h-2" />
                            <path d="M7 21H5a2 2 0 0 1-2-2v-2" />
                        </svg>
                    </label>
                    <label className="enable-grid-mode">
                        <input type="checkbox"
                            checked={props.gridModeEnabled}
                            onChange={event => {
                                excalidrawApi?.current?.updateScene({
                                    appState: {
                                        gridModeEnabled: event.target.checked,
                                    }
                                });
                            }}
                        />
                        <svg xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            className="icon focus lucide"
                        >
                            <rect width="18" height="18" x="3" y="3" rx="2" />
                            <path d="M3 9h18" />
                            <path d="M3 15h18" />
                            <path d="M9 3v18" />
                            <path d="M15 3v18" />
                        </svg>
                    </label>
                    <label className="lock-scroll-position">
                        <input type="checkbox"
                            checked={props.lockScrollPos != null}
                            onChange={event => {
                                if (!excalidrawApi?.current) {
                                    return void console.warn(`Backend not ready ..`);
                                }
                                
                                const [lockScrollPosition, setLockScrollPosition] = props.lockScrollPos ?? [];

                                if (lockScrollPosition != undefined) {
                                    return void setLockScrollPosition?.call(null, undefined);
                                }

                                const appState = excalidrawApi.current.getAppState();
                                const elements = excalidrawApi.current.getSceneElements();
                                const scrollDuration = 200;

                                if (elements.length > 0) {
                                    const selectedElements = elements.filter(element => {
                                        return appState.selectedElementIds[element.id];
                                    });

                                    const scrollTargets = selectedElements.length > 0
                                        ? selectedElements
                                        : elements;

                                    excalidrawApi.current.scrollToContent(scrollTargets, {
                                        fitToContent: true,
                                        animate: true,
                                        duration: scrollDuration,
                                    });

                                    setTimeout(() => {
                                        setLockScrollPosition?.call(null, [appState.scrollX, appState.scrollY, appState.zoom.value]);
                                    }, scrollDuration);
                                }
                            }}
                        />
                        <svg xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            className="icon magnet lucide"
                        >
                            <path d="m12 15 4 4" />
                            <path d="M2.352 10.648a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l6.029-6.029a1 1 0 1 1 3 3l-6.029 6.029a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l6.365-6.367A1 1 0 0 0 8.716 4.282z" />
                            <path d="m5 8 4 4" />
                        </svg>
                    </label>
                </section>
            </div>
        </div>
    )
}
