export * from "../.output/pkg/web/escher.js";

import init from "../.output/pkg/web/escher.js";

try {
    // TODO: Switch on environment to load the correct wasm stuff here ..
    const _wasm = await init();
    
    // TODO: Additional setup?
} catch (error) {
    // TODO
    console.error(`Failed to load wasm:`, error);
}
