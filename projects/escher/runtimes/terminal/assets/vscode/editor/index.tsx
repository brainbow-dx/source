// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Excalidraw } from "@excalidraw/excalidraw";
import * as excalidraw from "@excalidraw/excalidraw";

import App from "@escher/web/components/workspace";

const rootElement = createRoot(document.getElementById("root")!);

rootElement.render(
  <StrictMode>
    <App
      displayName={"Sketch"}
      useCustom={(api: any, args?: any[]) => { }}
      excalidrawLib={excalidraw}
    >
      <Excalidraw />
    </App>
  </StrictMode>,
);