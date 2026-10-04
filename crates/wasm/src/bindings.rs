//! Typed browser viewer. Alef binds this implementation directly, without a backend wrapper.

#[cfg(target_arch = "wasm32")]
use crate::wasm::{WasmLogger, canvas, create_canvas};
use crate::{Animation, Scene};
use cosmol_viewer_core::Request;
#[cfg(target_arch = "wasm32")]
use cosmol_viewer_core::{
    App, Feedback,
    transport::{self, RendererTransport, ViewerTransport},
};
#[cfg(target_arch = "wasm32")]
use eframe::WebRunner;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::{cell::Cell, rc::Rc};

pub fn binding_contract_json() -> String {
    cosmol_viewer_core::binding_contract::to_json()
}

/// A browser renderer. Scene and animation arguments are typed, reusable handles.
#[derive(Clone)]
pub struct Viewer {
    #[cfg(target_arch = "wasm32")]
    runner: WebRunner,
    #[cfg(target_arch = "wasm32")]
    owned_canvas: Rc<RefCell<Option<web_sys::HtmlCanvasElement>>>,
    #[cfg(target_arch = "wasm32")]
    transport: ViewerTransport,
    #[cfg(target_arch = "wasm32")]
    pending_renderer: Rc<RefCell<Option<RendererTransport>>>,
    request_id: Rc<Cell<u64>>,
    initialized: Rc<Cell<bool>>,
    closed: Rc<Cell<bool>>,
}

impl Viewer {
    /// Create an uninitialized viewer. Prefer render or play for normal use.
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();
        #[cfg(target_arch = "wasm32")]
        let (transport, renderer) = transport::pair();
        Self {
            #[cfg(target_arch = "wasm32")]
            runner: WebRunner::new(),
            #[cfg(target_arch = "wasm32")]
            owned_canvas: Rc::new(RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            transport,
            #[cfg(target_arch = "wasm32")]
            pending_renderer: Rc::new(RefCell::new(Some(renderer))),
            request_id: Rc::new(Cell::new(0)),
            initialized: Rc::new(Cell::new(false)),
            closed: Rc::new(Cell::new(false)),
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
            *viewer.owned_canvas.borrow_mut() = Some(canvas);
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
            *viewer.owned_canvas.borrow_mut() = Some(canvas);
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

    /// Queue a typed scene update. The renderer applies it on a subsequent frame.
    pub fn update(&self, scene: &Scene) -> Result<(), String> {
        self.ensure_initialized("Viewer update received before app initialization")?;
        self.send_request(Request::UpdateScene {
            request_id: self.next_request_id(),
            scene: scene.snapshot_for_render(),
        })
    }

    pub fn camera_parameter_logging(&self, enabled: bool) -> Result<(), String> {
        self.ensure_initialized("Camera logging received before app initialization")?;
        self.send_request(Request::CameraParameterLogging {
            request_id: self.next_request_id(),
            enabled,
        })
    }

    /// Show renderer FPS. One diagnostic repaint shows zero after one second idle,
    /// without counting that repaint or renewing the idle timer.
    pub fn show_fps(&self, enabled: bool) -> Result<(), String> {
        self.ensure_initialized("FPS overlay received before app initialization")?;
        self.send_request(Request::ShowFps {
            request_id: self.next_request_id(),
            enabled,
        })
    }

    /// Show live camera parameters over the viewport, independently of logging.
    pub fn show_camera_parameters(&self, enabled: bool) -> Result<(), String> {
        self.ensure_initialized("Camera overlay received before app initialization")?;
        self.send_request(Request::ShowCameraParameters {
            request_id: self.next_request_id(),
            enabled,
        })
    }

    /// Stop rendering. Auto-created canvases are removed; caller-owned canvases remain.
    pub fn close(&self) {
        self.closed.set(true);
        self.initialized.set(false);
        #[cfg(target_arch = "wasm32")]
        {
            self.transport.close();
            self.pending_renderer.borrow_mut().take();
            self.runner.destroy();
            if let Some(canvas) = self.owned_canvas.borrow_mut().take() {
                canvas.remove();
            }
        }
    }

    /// Return PNG bytes to JavaScript. Notebook response transport is not supported.
    pub async fn take_screenshot(&self) -> Result<Vec<u8>, String> {
        self.ensure_initialized("Screenshot requested before app initialization")?;
        #[cfg(target_arch = "wasm32")]
        {
            let request_id = self.next_request_id();
            self.send_request(Request::TakeScreenshot { request_id })?;
            let Feedback::ScreenshotTaken {
                width,
                height,
                rgba,
                ..
            } = self.transport.wait_screenshot(request_id).await?
            else {
                return Err("Unexpected screenshot feedback".into());
            };
            let image = image::RgbaImage::from_raw(width, height, rgba)
                .ok_or("Invalid screenshot dimensions or data")?;
            let mut bytes = Vec::new();
            image
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .map_err(|error| error.to_string())?;
            Ok(bytes)
        }
        #[cfg(not(target_arch = "wasm32"))]
        Err("Browser rendering requires wasm32 and a DOM canvas".into())
    }

    fn ensure_initialized(&self, message: &str) -> Result<(), String> {
        if self.closed.get() {
            Err("Viewer is closed".into())
        } else if !self.initialized.get() {
            Err(message.into())
        } else {
            Ok(())
        }
    }

    fn next_request_id(&self) -> u64 {
        let id = self.request_id.get() + 1;
        self.request_id.set(id);
        id
    }

    fn send_request(&self, request: Request) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            self.transport.send_request(request)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = request;
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn start_scene(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        scene: &Scene,
    ) -> Result<(), String> {
        let width = canvas.width() as f32;
        let height = canvas.height() as f32;
        self.start_request(
            canvas,
            Request::InitializeScene {
                request_id: self.next_request_id(),
                scene: scene.snapshot_for_render(),
                width,
                height,
            },
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    async fn start_request(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        request: Request,
    ) -> Result<(), String> {
        self.send_request(request)?;
        let transport = self
            .pending_renderer
            .borrow_mut()
            .take()
            .ok_or("Viewer renderer already started")?;
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    let request = transport
                        .try_request()
                        .unwrap()
                        .expect("Missing initialization request");
                    Ok(Box::new(App::from_request(
                        cc, request, WasmLogger, transport,
                    )))
                }),
            )
            .await
            .map_err(|error| error.as_string().unwrap_or_else(|| format!("{error:?}")))?;
        self.initialized.set(true);
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    async fn start_animation(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        animation: &Animation,
    ) -> Result<(), String> {
        let width = canvas.width() as f32;
        let height = canvas.height() as f32;
        self.start_request(
            canvas,
            Request::InitializeAnimation {
                request_id: self.next_request_id(),
                animation: animation.snapshot_for_render(),
                width,
                height,
            },
        )
        .await
    }
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new()
    }
}
