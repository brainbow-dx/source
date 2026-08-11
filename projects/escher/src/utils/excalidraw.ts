// import { useContext, createContext } from "react";

// import type { ExcalidrawImperativeAPI } from "@excalidraw/excalidraw/types";

// import type { MutableState } from "#src/components/editor/editor.tsx";

// /**
//  * TODO
//  */
// export const ExcalidrawContext = createContext<MutableState<ExcalidrawImperativeAPI>>([undefined, undefined]);

// export function useExcalidraw() {
//     const context = useContext(ExcalidrawContext);

//     if (context === undefined) {
//         throw new Error("useExcalidraw must be used within an ExcalidrawProvider");
//     }

//     return context;
// }
