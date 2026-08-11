import { newElementWith } from "@excalidraw/excalidraw";
import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";
import type { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";

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

export function createVideoEmbed(excalidrawApi: ExcalidrawImperativeAPI, element: Partial<ExcalidrawElement>): void {
    excalidrawApi?.updateScene({
        elements: [
            ...excalidrawApi.getSceneElementsIncludingDeleted(),
            newElementWith({
                ...NEW_VIDEO_ELEMENT,
                id: `id-${Math.floor(Math.random() * 100000)}`,
            }, element),
        ]
    });
}
