//! Browser-specific canvas and eframe integration. No transport decoding lives here.

use cosmol_viewer_core::utils::Logger;
use wasm_bindgen::JsValue;
#[derive(Clone, Copy)]
pub(crate) struct WasmLogger;
impl Logger for WasmLogger {
    fn log(&self, message: impl std::fmt::Display) {
        web_sys::console::log_1(&JsValue::from_str(&message.to_string()));
    }
    fn warn(&self, message: impl std::fmt::Display) {
        web_sys::console::warn_1(&JsValue::from_str(&message.to_string()));
    }
    fn error(&self, message: impl std::fmt::Display) {
        let message = message.to_string();
        web_sys::console::error_1(&JsValue::from_str(&message));
        if let Some(window) = web_sys::window() {
            window.alert_with_message(&message).ok();
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn canvas(id: &str) -> Result<web_sys::HtmlCanvasElement, String> {
    use wasm_bindgen::JsCast;
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .ok_or_else(|| format!("Canvas not found: {id}"))?
        .dyn_into()
        .map_err(|_| format!("Element is not a canvas: {id}"))
}
