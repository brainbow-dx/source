// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import Workspace from "@brainbow/slate-terminal/views/workspace";

//---
const rootElement = createRoot(document.getElementById("root")!);

rootElement.render(
  <StrictMode>
    <Workspace displayName="Sketch" />
  </StrictMode>,
);
