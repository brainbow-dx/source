use wasm_bindgen::prelude::*;

//---
// #[cwrap(C, Ecma, CSharp, Cpp, Web)]
#[wasm_bindgen]
#[repr(C)]
pub struct WebSurface {
    styleguide: JsValue,
}

#[wasm_bindgen(js_name="showNotification")]
#[no_mangle(name="show_notification")]
pub fn c_show_notification(message: &str) {
    // TODO: Show notifications ..
    alert(&format!("Should only run in browser: {}", message));
}

//---
#[wasm_bindgen]
unsafe extern "C" {
    pub fn alert(s: &str);
}
