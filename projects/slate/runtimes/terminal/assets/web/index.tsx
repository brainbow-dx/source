// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import Workspace from "../../../src/views/workspace/workspace.tsx";

//---
const rootElement = createRoot(document.getElementById("root")!);

rootElement.render(
  <StrictMode>
    <Workspace displayName="Sketch" />
  </StrictMode>,
);
