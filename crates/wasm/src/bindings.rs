//! Typed browser viewer. Alef binds this implementation directly, without a backend wrapper.

use crate::wasm::WasmLogger;
#[cfg(target_arch = "wasm32")]
use crate::wasm::{WebApp, canvas, create_canvas};
use crate::{Animation, Scene};
use cosmol_viewer_core::App;
#[cfg(target_arch = "wasm32")]
use eframe::WebRunner;
use std::sync::{Arc, Mutex};

pub fn binding_contract_json() -> String {
    cosmol_viewer_core::binding_contract::to_json()
}

/// A browser renderer. Scene and animation arguments are typed, reusable handles.
#[derive(Clone)]
pub struct Viewer {
    #[cfg(target_arch = "wasm32")]
    runner: WebRunner,
    #[cfg(target_arch = "wasm32")]
    owned_canvas: Arc<Mutex<Option<web_sys::HtmlCanvasElement>>>,
    app: Arc<Mutex<Option<App<WasmLogger>>>>,
}

impl Viewer {
    /// Create an uninitialized viewer. Prefer render or play for normal use.
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();
        Self {
            #[cfg(target_arch = "wasm32")]
            runner: WebRunner::new(),
            #[cfg(target_arch = "wasm32")]
            owned_canvas: Arc::new(Mutex::new(None)),
            app: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a canvas in document.body and render a typed scene into it.
    pub async fn render(scene: &Scene, width: f32, height: f32) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let canvas = create_canvas(width, height)?;
            let viewer = Self::new();
            if let Err(error) = viewer.start_scene(canvas.clone(), scene).await {
                canvas.remove();
                return Err(error);
            }
            *viewer.owned_canvas.lock().unwrap() = Some(canvas);
            Ok(viewer)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (scene, width, height);
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    /// Render into an existing canvas. The caller owns canvas sizing and mounting.
    pub async fn render_into(canvas_id: String, scene: &Scene) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let viewer = Self::new();
            viewer.start_scene(canvas(&canvas_id)?, scene).await?;
            Ok(viewer)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (canvas_id, scene);
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    /// Create a canvas in document.body and play a typed animation.
    pub async fn play(animation: &Animation, width: f32, height: f32) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let canvas = create_canvas(width, height)?;
            let viewer = Self::new();
            if let Err(error) = viewer.start_animation(canvas.clone(), animation).await {
                canvas.remove();
                return Err(error);
            }
            *viewer.owned_canvas.lock().unwrap() = Some(canvas);
            Ok(viewer)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (animation, width, height);
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    /// Play a typed animation in an existing canvas without consuming it.
    pub async fn play_into(canvas_id: String, animation: &Animation) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let viewer = Self::new();
            viewer
                .start_animation(canvas(&canvas_id)?, animation)
                .await?;
            Ok(viewer)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (canvas_id, animation);
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    /// Update the renderer directly from a typed scene; no transport payload is involved.
    pub fn update(&self, scene: &Scene) -> Result<(), String> {
        let scene = scene.snapshot_for_render();
        let mut guard = self.app.lock().unwrap();
        let app = guard
            .as_mut()
            .ok_or("Viewer update received before app initialization")?;
        app.update_scene(&scene);
        app.ctx.request_repaint();
        Ok(())
    }

    pub fn set_camera_parameter_logging(&self, enabled: bool) -> Result<(), String> {
        let mut guard = self.app.lock().unwrap();
        let app = guard
            .as_mut()
            .ok_or("Camera logging received before app initialization")?;
        app.set_camera_parameter_logging(enabled);
        Ok(())
    }

    /// Stop rendering. Auto-created canvases are removed; caller-owned canvases remain.
    pub fn close(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.runner.destroy();
            if let Some(canvas) = self.owned_canvas.lock().unwrap().take() {
                canvas.remove();
            }
        }
        self.app.lock().unwrap().take();
    }

    /// Return PNG bytes, not a notebook transport envelope.
    pub async fn take_screenshot(&self) -> Result<Vec<u8>, String> {
        {
            let mut guard = self.app.lock().unwrap();
            let app = guard
                .as_mut()
                .ok_or("Screenshot requested before app initialization")?;
            app.take_screenshot();
            app.ctx.request_repaint();
        }
        loop {
            {
                let mut guard = self.app.lock().unwrap();
                let app = guard
                    .as_mut()
                    .ok_or("Viewer closed while awaiting screenshot")?;
                if let Some(image) = app.poll_screenshot() {
                    let mut bytes = Vec::new();
                    image
                        .write_to(
                            &mut std::io::Cursor::new(&mut bytes),
                            image::ImageFormat::Png,
                        )
                        .map_err(|error| error.to_string())?;
                    return Ok(bytes);
                }
            }
            gloo_timers::future::TimeoutFuture::new(100).await;
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn start_scene(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        scene: &Scene,
    ) -> Result<(), String> {
        let scene = scene.snapshot_for_render();
        let state = self.app.clone();
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    *state.lock().unwrap() = Some(App::new(cc, &scene, WasmLogger));
                    Ok(Box::new(WebApp(state.clone())))
                }),
            )
            .await
            .map_err(|error| error.as_string().unwrap_or_else(|| format!("{error:?}")))
    }

    #[cfg(target_arch = "wasm32")]
    async fn start_animation(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        animation: &Animation,
    ) -> Result<(), String> {
        let animation = animation.snapshot_for_render();
        let state = self.app.clone();
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    *state.lock().unwrap() = Some(App::new_play(cc, animation, WasmLogger));
                    Ok(Box::new(WebApp(state.clone())))
                }),
            )
            .await
            .map_err(|error| error.as_string().unwrap_or_else(|| format!("{error:?}")))
    }
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}
