// TODO: Remove this:
// deno-lint-ignore-file
import { useCallback, useContext, useRef } from "react";
import type { MouseEventHandler, PointerEventHandler } from "react";

import { newElementWith } from "@excalidraw/excalidraw";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";
import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";

import { EditorContext, ExcalidrawContext } from "#src/components/editor/editor.tsx";
import type { ActiveEmbeddable } from "#src/components/editor/editor.tsx";

const NEW_VIDEO_ELEMENT: ExcalidrawElement = {
    id: "<unknown>",
    updated: Date.now(),
    isDeleted: false,
    locked: false,
    version: 1,
    versionNonce: 0,
    seed: Math.floor(Math.random() * 100000),
    index: null, // TODO: After selected element's index?
    groupIds: [],
    frameId: null,
    type: "embeddable",
    link: null,
    x: 0, // TODO: Get from current scene location.
    y: 0, // TODO: Get from current scene location.
    width: 600,
    height: 480,
    backgroundColor: "transparent",
    fillStyle: "solid",
    strokeWidth: 4,
    strokeStyle: "dashed",
    strokeColor: "#6741d9",
    opacity: 100,
    roundness: {
        type: 3,
    },
    angle: null,
    roughness: 0,
    boundElements: [
        //..
    ],
    customData: {
        customKey: "TODO",
        settings: {
            test01: "TODO",
        }
    },
};

export function createElement(excalidrawApi: ExcalidrawImperativeAPI, partial: any) {
}

export interface ElementControlPanelProps {
    activeEmbeddable?: ActiveEmbeddable,
    selectedGroupIds?: {[key: string]: boolean},
    selectedElementIds?: {[key: string]: boolean},
}

export function ElementControlPanel(props: ElementControlPanelProps) {
    const [editor] = useContext(EditorContext);
    const excalidrawApi = useContext(ExcalidrawContext);
    
    const createButton = useRef<HTMLButtonElement>(null);
    const borderColorSelect = useRef<HTMLSelectElement>(null);
    
    // console.debug(`TODO: Scene Editor Panel (Control Sidebar Header):`);
    
    // TODO:
    //   If I have selected an Embed, show the resource control header.
    //   If not, fallback in this order:
    //     a. Selected Group(s)
    //     b. Selected Element(s)
    //     c. Active Scene
    
    const onClick = useCallback<MouseEventHandler<HTMLButtonElement>>(event => {
        if (!excalidrawApi?.current) {
            return void console.warn(`Unable to create new element; Excalidraw api not ready.`);
        }
        
        const groupIds = props.selectedGroupIds
            ? Object.keys(props.selectedGroupIds)
            : []; // TODO
        
        const strokeColor = borderColorSelect.current?.value;
        console.debug(`Border Color:`, strokeColor);
        
        excalidrawApi?.current?.updateScene({
            elements: [
                ...excalidrawApi?.current.getSceneElementsIncludingDeleted(),
                newElementWith({
                    ...NEW_VIDEO_ELEMENT,
                    id: `id-${Math.floor(Math.random() * 100000)}`,
                }, {
                    groupIds, // TODO: [default outer-selection groupId] ..
                    strokeColor,
                    // link: "https://www.youtube.com/embed/dQw4w9WgXcQ",
                }),
            ]
        });
    }, [
        borderColorSelect,
    ]);
    
    return (
        <div role="tabpanel" className="sidebar-panel controls">
            <select ref={borderColorSelect}>
                <option value="#1971c2">Blue</option>
                <option value="#099268">Green</option>
                <option value="#6741d9">Purple</option>
            </select>
            <button type="button" ref={createButton} onClick={onClick}>
                Create new element!
            </button>
        </div>
    )
}
