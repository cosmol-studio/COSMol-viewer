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
pub(crate) struct WebApp(
    pub std::sync::Arc<std::sync::Mutex<Option<cosmol_viewer_core::App<WasmLogger>>>>,
);
#[cfg(target_arch = "wasm32")]
impl eframe::App for WebApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        if let Some(app) = &mut *self.0.lock().unwrap() {
            app.ui(ui, frame);
        }
    }
    fn clear_color(&self, visuals: &eframe::egui::Visuals) -> [f32; 4] {
        self.0
            .lock()
            .unwrap()
            .as_ref()
            .map(|app| app.clear_color(visuals))
            .unwrap_or([0.0; 4])
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
#[cfg(target_arch = "wasm32")]
pub(crate) fn create_canvas(width: f32, height: f32) -> Result<web_sys::HtmlCanvasElement, String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    use wasm_bindgen::JsCast;
    static NEXT_CANVAS: AtomicU32 = AtomicU32::new(0);
    if !width.is_finite()
        || !height.is_finite()
        || width < 1.0
        || height < 1.0
        || width > u32::MAX as f32
        || height > u32::MAX as f32
    {
        return Err("Canvas dimensions must be finite, positive pixel sizes".into());
    }
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("Browser document is unavailable")?;
    let body = document.body().ok_or("Document body is unavailable")?;
    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(|error| format!("Cannot create canvas: {error:?}"))?
        .dyn_into()
        .map_err(|_| "Cannot create an HTML canvas".to_string())?;
    let id = loop {
        let id = format!(
            "cosmol-viewer-canvas-{}",
            NEXT_CANVAS.fetch_add(1, Ordering::Relaxed)
        );
        if document.get_element_by_id(&id).is_none() {
            break id;
        }
    };
    canvas.set_id(&id);
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);
    canvas
        .set_attribute("style", &format!("width:{width}px;height:{height}px;"))
        .map_err(|error| format!("Cannot size canvas: {error:?}"))?;
    body.append_child(&canvas)
        .map_err(|error| format!("Cannot mount canvas: {error:?}"))?;
    Ok(canvas)
}
