// TODO: Remove this:
// deno-lint-ignore-file
import { useRef, useCallback, useContext, useState } from "react";
import type { MouseEventHandler, FormEventHandler } from "react";

import { newElementWith } from "@excalidraw/excalidraw";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";
import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";

import { createVideoEmbed } from "#src/external/excalidraw/elements/video.tsx";

import { EditorContext, ExcalidrawContext, useExcalidraw } from "#src/components/editor/editor.tsx";
import type { ActiveEmbeddable } from "#src/components/editor/editor.tsx";

export interface SceneOverviewPanelProps {
    activeEmbeddable?: ActiveEmbeddable,
    selectedGroupIds?: {[key: string]: boolean},
    selectedElementIds?: {[key: string]: boolean},
}

export function SceneOverviewPanel(props: SceneOverviewPanelProps) {
    const [editor] = useContext(EditorContext);
    const excalidrawApi = useExcalidraw();
    
    if (!excalidrawApi?.current) {
        return (
            <section style={{ margin: "0 1rem" }}>
                ... loading api ..
            </section>
        )
    }
    
    const [name, setName] = useState(excalidrawApi.current.getName());
    
    const onInput = useCallback<FormEventHandler<HTMLInputElement>>((event) => {
        if (!excalidrawApi?.current) {
            return void console.warn(`Failed to capture input. Excalidraw api not ready.`);
        }
        if (excalidrawApi.current.getName() != event.currentTarget.value) {
            console.debug(`Handle Form Event:`, event);
            setName(event.currentTarget.value);
            excalidrawApi.current.updateScene({
                appState: {
                    name: event.currentTarget.value,
                },
                captureUpdate: "IMMEDIATELY",
            })
        }
    }, [
        name,
    ]);
    
    // TODO:
    //   If I have selected an Embed, show the resource control header.
    //   If not, fallback in this order:
    //     a. Selected Group(s)
    //     b. Selected Element(s)
    //     c. Active Scene
    
    return (
        <div className="scene-overview">
            <header>
                <h3>
                    {excalidrawApi.current.getName()}
                </h3>
                <input type="text"
                    onInput={onInput}
                    value={excalidrawApi.current.getName()}
                />
            </header>
            <div>
                {props.activeEmbeddable && (
                    <section>
                        <h5>Active Embeddable:</h5>
                        <input type="text" disabled={true} value={props.activeEmbeddable.element.id} />
                        {props.activeEmbeddable.state == "active" && (
                            <pre style={{ overflow: "hidden" }}>
                                
                            </pre>
                        )}
                    </section>
                )}
                {props.selectedGroupIds && (
                    <section>
                        <h5>Selected Groups:</h5>
                        <pre>
                            <pre style={{ overflow: "hidden" }}>
                                {JSON.stringify(props.selectedGroupIds, null, 2)}
                            </pre>
                        </pre>
                    </section>
                )}
                {props.selectedElementIds && (
                    <section>
                        <h5>Selected Elements:</h5>
                        <cite>
                            TODO: Slkjsdf
                        </cite>
                        {excalidrawApi.current.getSceneElements().map(element => {
                            if (!props.selectedElementIds) {
                                return null;
                            }
                            return Object.keys(props.selectedElementIds).includes(element.id) && (
                                // Selected element row display ..
                                <div key={element.id}>
                                    {element.id}
                                </div>
                            )
                        })}
                    </section>
                )}
                <footer>
                    <NewElementPanel
                        activeEmbeddable={props.activeEmbeddable}
                        selectedGroupIds={props.selectedGroupIds}
                        selectedElementIds={props.selectedElementIds}
                    />
                </footer>
            </div>
        </div>
    )
}

export interface NewElementPanelProps {
    activeEmbeddable?: ActiveEmbeddable,
    selectedGroupIds?: {[key: string]: boolean},
    selectedElementIds?: {[key: string]: boolean},
}

export function NewElementPanel(props: NewElementPanelProps) {
    const [editor] = useContext(EditorContext);
    const excalidrawApi = useContext(ExcalidrawContext);
    
    const [strokeColor, setStrokeColor] = useState("red");
    
    // const createButton = useRef<HTMLButtonElement>(null);
    // const strokeColorSelect = useRef<HTMLSelectElement>(null);
    
    const onClick = useCallback<MouseEventHandler<HTMLButtonElement>>(event => {
        if (!excalidrawApi) {
            return console.warn(`Excalidraw API not available ..`);
        }
        
        console.debug(`Border Color:`, strokeColor);
        if (excalidrawApi?.current) {
            createVideoEmbed(excalidrawApi.current, {
                strokeColor,
                groupIds: Object.keys(props.selectedGroupIds ?? {
                    // TODO: Default group ids??
                }),
            });
        }
    }, [
        strokeColor,
    ]);
    
    return (
        <div role="dialog" className="new-element-panel">
            <header>
                <h3>Create New Element</h3>
            </header>
            <section>
                <fieldset>
                    <legend>Stroke</legend>
                    <select value={strokeColor} onChange={event => {
                        setStrokeColor(event.currentTarget.value);
                    }}>
                        <option value="#1971c2">Blue</option>
                        <option value="#099268">Green</option>
                        <option value="#6741d9">Purple</option>
                    </select>
                </fieldset>
                <fieldset>
                    <legend>Stroke</legend>
                    <select value={strokeColor} onChange={event => {
                        console.debug(`onChange:`, event.target.value);
                        setStrokeColor(event.currentTarget.value);
                    }}>
                        <option value="#1971c2">Blue</option>
                        <option value="#099268">Green</option>
                        <option value="#6741d9">Purple</option>
                    </select>
                </fieldset>
            </section>
            <footer>
                <fieldset>
                    <button type="button" onClick={onClick}>
                        Create new element!
                    </button>
                </fieldset>
            </footer>
        </div>
    )
}
