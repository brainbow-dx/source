// deno-lint-ignore-file
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import Workspace from "@escher/web/components/workspace";

//---
const rootElement = createRoot(document.getElementById("root")!);

rootElement.render(
  <StrictMode>
    <Workspace displayName="Sketch" />
  </StrictMode>,
);
