// Registers <escher-scaffold>, a web component that mounts an escher_core::Scaffold tree
// (rendered by escher-web's wasm build, see src/surface.rs/description.rs) into its own DOM
// subtree. Plain, dependency-free ESM — copied as-is into .output/pkg/web/ by scripts/build.tsx,
// loaded directly by the browser, no bundling needed.
//
// A nested <script type="application/json"> child, if present, is the JSON output of an
// @escher/jsx-authored page (see packages/jsx) — mounted via mountScaffoldFromJson. Without one,
// falls back to the built-in placeholder scaffold (mountScaffold).
import init, { mountScaffold, mountScaffoldFromJson } from "./escher.js";

let ready;

function ensureInit() {
    if (!ready) {
        ready = init();
    }
    return ready;
}

class EscherScaffoldElement extends HTMLElement {
    async connectedCallback() {
        await ensureInit();

        const payload = this.querySelector('script[type="application/json"]');
        if (payload) {
            mountScaffoldFromJson(this, payload.textContent);
        } else {
            mountScaffold(this);
        }
    }
}

customElements.define("escher-scaffold", EscherScaffoldElement);
