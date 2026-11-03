//! Typed browser viewer exported directly through wasm-bindgen.
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
use crate::wasm::{WasmLogger, canvas};
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

#[cfg_attr(all(feature = "wasm", target_arch = "wasm32"), wasm_bindgen(js_name = bindingContractJson))]
pub fn binding_contract_json() -> String {
    cosmol_viewer_core::binding_contract::to_json()
}

/// A browser renderer. Scene and animation arguments are typed, reusable handles.
#[derive(Clone)]
#[cfg_attr(all(feature = "wasm", target_arch = "wasm32"), wasm_bindgen)]
pub struct Viewer {
    #[cfg(target_arch = "wasm32")]
    runner: WebRunner,
    #[cfg(target_arch = "wasm32")]
    canvas: web_sys::HtmlCanvasElement,
    #[cfg(target_arch = "wasm32")]
    transport: ViewerTransport,
    #[cfg(target_arch = "wasm32")]
    pending_renderer: Rc<RefCell<Option<RendererTransport>>>,
    request_id: Rc<Cell<u64>>,
    initialized: Rc<Cell<bool>>,
    starting: Rc<Cell<bool>>,
    closed: Rc<Cell<bool>>,
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
impl crate::input::NativeOutput for Viewer {}

#[cfg_attr(
    all(feature = "wasm", target_arch = "wasm32"),
    cosmol_viewer_derive::checked_wasm
)]
impl Viewer {
    /// Bind a viewer to an existing canvas ID without starting rendering.
    /// The caller owns canvas placement and dimensions.
    pub fn new(canvas_id: String) -> Result<Self, String> {
        #[cfg(target_arch = "wasm32")]
        {
            let canvas = canvas(&canvas_id)?;
            eframe::WebLogger::init(log::LevelFilter::Debug).ok();
            let (transport, renderer) = transport::pair();
            Ok(Self {
                runner: WebRunner::new(),
                canvas,
                transport,
                pending_renderer: Rc::new(RefCell::new(Some(renderer))),
                request_id: Rc::new(Cell::new(0)),
                initialized: Rc::new(Cell::new(false)),
                starting: Rc::new(Cell::new(false)),
                closed: Rc::new(Cell::new(false)),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = canvas_id;
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    /// Render a scene on this viewer's canvas, stopping any previous animation.
    /// First use starts the renderer; subsequent calls queue a new scene.
    pub async fn render(&self, scene: &Scene) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            self.start_scene(self.canvas.clone(), scene).await
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = scene;
            Err("Browser rendering requires wasm32 and a DOM canvas".into())
        }
    }

    /// Play an animation on this viewer's canvas, replacing previous content.
    pub async fn play(&self, animation: &Animation) -> Result<(), String> {
        if animation.frame_count() == 0 {
            return Err("Animation must have at least one frame".into());
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.start_animation(self.canvas.clone(), animation).await
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = animation;
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

    /// Stop rendering and release renderer resources. The caller-owned canvas remains.
    pub fn close(&self) {
        self.closed.set(true);
        self.initialized.set(false);
        #[cfg(target_arch = "wasm32")]
        {
            self.transport.close();
            self.pending_renderer.borrow_mut().take();
            self.runner.destroy();
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
}

impl Viewer {
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
        if self.closed.get() {
            return Err("Viewer is closed".into());
        }
        if self.initialized.get() {
            return self.send_request(request);
        }
        if self.starting.replace(true) {
            return Err("Viewer initialization is already in progress".into());
        }
        self.send_request(request)?;
        let transport = self
            .pending_renderer
            .borrow_mut()
            .take()
            .ok_or("Viewer renderer already started")?;
        let result = self
            .runner
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
            .map_err(|error| error.as_string().unwrap_or_else(|| format!("{error:?}")));
        self.starting.set(false);
        if let Err(error) = result {
            self.close();
            return Err(error);
        }
        if self.closed.get() {
            self.runner.destroy();
            return Err("Viewer closed during initialization".into());
        }
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
