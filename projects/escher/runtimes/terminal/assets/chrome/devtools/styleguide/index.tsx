// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Excalidraw } from "@excalidraw/excalidraw";

import App from "@escher/web/components/workspace";

const rootElement = createRoot(document.getElementById("root")!);

rootElement.render(
  <StrictMode>
    <App
      displayName={"Sketch"}
    >
      <Excalidraw />
    </App>
  </StrictMode>,
);