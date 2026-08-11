import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { sendDebugPayload } from "#src/mod.ts";

import "#assets/index.css";

//---
const app = document.getElementById("app");
if (app?.classList?.contains("surface")) {
    createRoot(app).render(
        <StrictMode>
            <section onClick={_event => sendDebugPayload("Clicked!??")}>
                <h1>Some Thing</h1>
                <p>Whatever man ..</p>
            </section>
        </StrictMode>,
    );
}