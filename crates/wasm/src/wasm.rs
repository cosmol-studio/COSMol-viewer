use crate::utils::compress_data;
#[cfg(target_arch = "wasm32")]
use crate::utils::decompress_animation;
use crate::utils::decompress_data;
use cosmol_viewer_core::scene::Scene;
#[cfg(target_arch = "wasm32")]
use eframe::WebRunner;
use {
    cosmol_viewer_core::{App, utils::Logger},
    std::sync::{Arc, Mutex},
    wasm_bindgen::{JsValue, prelude::wasm_bindgen},
    web_sys::HtmlCanvasElement,
};

#[derive(Clone, Copy)]
pub struct WasmLogger;

impl Logger for WasmLogger {
    fn log(&self, message: impl std::fmt::Display) {
        web_sys::console::log_1(&JsValue::from_str(&message.to_string()));
    }

    fn warn(&self, message: impl std::fmt::Display) {
        web_sys::console::warn_1(&JsValue::from_str(&message.to_string()));
    }

    fn error(&self, message: impl std::fmt::Display) {
        let msg = message.to_string();

        // Send to console
        web_sys::console::error_1(&JsValue::from_str(&msg));

        // Show browser alert
        if let Some(window) = web_sys::window() {
            window.alert_with_message(&msg).ok();
        }
    }
}

#[wasm_bindgen]
pub struct WebHandle {
    #[cfg(target_arch = "wasm32")]
    runner: WebRunner,
    app: Arc<Mutex<Option<App<WasmLogger>>>>,
    wasm_logger: WasmLogger,
}

// WebHandle and WebRunner still share the app for JavaScript updates and screenshots.
#[cfg(target_arch = "wasm32")]
struct WebApp(Arc<Mutex<Option<App<WasmLogger>>>>);

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
            .unwrap_or([0.0, 0.0, 0.0, 0.0])
    }
}

#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();
        Self {
            #[cfg(target_arch = "wasm32")]
            runner: WebRunner::new(),
            app: Arc::new(Mutex::new(None)),
            wasm_logger: WasmLogger,
        }
    }

    #[wasm_bindgen]
    pub async fn start_with_scene(
        &mut self,
        _canvas: HtmlCanvasElement,
        _scene_json: String,
    ) -> Result<(), JsValue> {
        #[cfg(target_arch = "wasm32")]
        {
            let scene: Scene =
                decompress_data(&_scene_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
            let app = Arc::clone(&self.app);

            let _ = self
                .runner
                .start(
                    _canvas,
                    eframe::WebOptions {
                        ..Default::default()
                    },
                    Box::new(move |cc| {
                        let mut guard = app.lock().unwrap();
                        *guard = Some(App::new(cc, &scene, WasmLogger));
                        Ok(Box::new(WebApp(app.clone())))
                    }),
                )
                .await;
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn update_scene(&mut self, scene_json: String) -> Result<(), JsValue> {
        let scene: Scene =
            decompress_data(&scene_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mut app_guard = self.app.lock().unwrap();
        if let Some(app) = &mut *app_guard {
            app.update_scene(&scene);
            app.ctx.request_repaint();
        } else {
            self.wasm_logger
                .warn("scene update received but app is not initialized");
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn initiate_viewer_and_play(
        &mut self,
        _canvas: HtmlCanvasElement,
        _animation_compressed: String,
    ) -> Result<(), JsValue> {
        #[cfg(target_arch = "wasm32")]
        {
            use cosmol_viewer_core::scene::Animation;

            let payload = _animation_compressed.to_string();
            let kb = payload.as_bytes().len() as f64 / 1024.0;
            web_sys::console::log_1(&format!("Transmission size: {kb:.2} KB").into());

            let animation: Animation = decompress_animation(&_animation_compressed)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let app = Arc::clone(&self.app);
            let _ = self
                .runner
                .start(
                    _canvas,
                    eframe::WebOptions::default(),
                    Box::new(move |cc| {
                        let mut guard = app.lock().unwrap();
                        *guard = Some(App::new_play(cc, animation, WasmLogger));
                        Ok(Box::new(WebApp(app.clone())))
                    }),
                )
                .await;
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn take_screenshot(&self) -> String {
        loop {
            let mut app_guard = self.app.lock().unwrap();
            if let Some(app) = &mut *app_guard {
                println!("Taking screenshot");
                app.take_screenshot();
                app.ctx.request_repaint();
                break;
            }
            drop(app_guard);
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        loop {
            let mut app_guard = self.app.lock().unwrap();
            if let Some(app) = &mut *app_guard {
                if let Some(image) = app.poll_screenshot() {
                    let mut buf = Vec::new();
                    self.wasm_logger.log(format!("image:{:?}", buf));
                    image
                        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                        .unwrap();
                    return compress_data(&buf).unwrap();
                }
            }
            drop(app_guard);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
