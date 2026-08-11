// TODO: Remove this:
// deno-lint-ignore-file
import { useContext } from "react";

import { EditorContext, ExcalidrawContext } from "#src/components/editor/editor.tsx";
import type { ActiveEmbeddable } from "#src/components/editor/editor.tsx";

import "../sidebar.css";

export interface ElementStylePanelProps {
    activeEmbeddable?: ActiveEmbeddable,
    selectedGroups?: {[key: string]: boolean},
    selectedElements?: {[key: string]: boolean},
}

export function ElementStylePanel(props: ElementStylePanelProps) {
    const [editor] = useContext(EditorContext);
    const [excalidrawApi] = useContext(ExcalidrawContext);
    
    // console.debug(`TODO: Scene Editor Panel (Control Sidebar Header):`);
    
    // TODO:
    //   If I have selected an Embed, show the resource control header.
    //   If not, fallback in this order:
    //     a. Selected Group(s)
    //     b. Selected Element(s)
    //     c. Active Scene
    
    return (
        <div className="inspector-panel" style={{ padding: "0.7rem", opacity: 0.2 }}>
            <header>
                <h3>Editor:</h3>
            </header>
            <section>
                <h3>Scene:</h3>
                <h5>Active Embeddable:</h5>
                {props.activeEmbeddable?.state == "active" && (
                    <pre style={{ overflow: "hidden" }}>
                        {JSON.stringify(props.activeEmbeddable, null)}
                    </pre>
                )}
                <h5>Selected Groups:</h5>
                <pre>
                    {JSON.stringify(props.selectedGroups, null, 2)}
                </pre>
            </section>
            {/*
                <header style={{ margin: "0 1rem" }}>
                    Inspect
                </header>
                <section style={{ overflowY: "auto" }}>
                    <section>
                        <ul>
                            <li>Backend <code>#{excalidrawApi?.id ?? "undefined"}</code></li>
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
                        <code>
                            <pre>
                                {JSON.stringify(activeEmbeddable, null, 2)}
                            </pre>
                        </code>
                    </section>
                    <section style={{ margin: "0 1rem" }}>
                        <h4>Selected Groups</h4>
                        <code>
                            <pre>
                                {excalidrawApi && Object.entries(selectedGroupIds).map(([groupId, _]) => {
                                    const sceneElements = excalidrawApi.getSceneElements();
                                    const groupElements = sceneElements.find(({ groupIds }) => groupIds.includes(groupId));
                                    
                                    return groupElements
                                        ? <li key={groupId} className="found">{groupId}</li>
                                        : <li key={groupId} className="missing">{groupId}</li>
                                })}
                            </pre>
                        </code>
                    </section>
                    <section style={{ margin: "0 1rem" }}>
                        <h4>Selected Elements</h4>
                        <code>
                            <pre>
                                {excalidrawApi && Object.entries(excalidrawApi.getAppState().selectedElementIds).map(([elementId, _]) => {
                                    const sceneElements = excalidrawApi.getSceneElements();
                                    const selectedElement = sceneElements?.find(({ id }) => id == elementId);
                                    
                                    return selectedElement
                                        ? <li key={elementId} className="found">{elementId}</li>
                                        : <li key={elementId} className="missing">{elementId}</li>
                                })}
                            </pre>
                        </code>
                    </section>
                    <section style={{ margin: "0 1rem" }}>
                        <h4>Exported Canvas</h4>
                        <div className="export export-canvas">
                            {canvasUrl ? (
                                <img src={canvasUrl} alt="" />
                            ) : (
                                <span>...</span>
                            )}
                        </div>
                    </section>
                </section>
            */}
            <footer>
                <h3>Elements:</h3>
                <h5>Selected Elements:</h5>
                {props.selectedElements && excalidrawApi?.getSceneElements().map(element => {
                    if (!props.selectedElements) {
                        return null;
                    }
                    
                    return Object.keys(props.selectedElements).includes(element.id) && (
                        <pre key={element.id}>
                            {JSON.stringify(element, null, 2)}
                        </pre>
                    )
                })}
            </footer>
        </div>
    )
}
