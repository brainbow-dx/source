#[cfg(all(target_arch="wasm32", target_os="unknown"))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch="wasm32", target_os="unknown"))]
#[wasm_bindgen(js_namespace="console")]
unsafe extern "C" {
    pub fn debug(s: &str);
    pub fn log(s: &str);
    pub fn info(s: &str);
    pub fn warning(s: &str);
    pub fn error(s: &str);
}

//---
#[cfg(all(target_arch="wasm32", target_os="unknown"))]
#[wasm_bindgen(js_name="sendDebugPayload")]
pub fn send_debug_payload(message: &str) {
    info(&format!("Can be called from anywhere: {}", message));
}
