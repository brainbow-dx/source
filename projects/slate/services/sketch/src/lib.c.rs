#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

//---
#[cfg(target_arch="wasm32")]
#[wasm_bindgen]
unsafe extern "C" {
    pub fn alert(s: &str);
}

//---
#[cfg(all(target_arch="wasm32", target_os="unknown"))]
#[wasm_bindgen]
pub fn init() {
    alert(&format!("TODO"));
}
