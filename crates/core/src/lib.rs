mod shader;
pub mod surface;
use crate::egui::IconData;
use iceoryx2::{
    node::{Node, NodeBuilder},
    port::{publisher::Publisher, subscriber::Subscriber},
    service::{ipc, port_factory::publish_subscribe::PortFactory},
};
use std::{
    cell::Cell,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub mod parser;
const VERSION: &str = env!("CARGO_PKG_VERSION");
static RENDER_REGISTERED: AtomicBool = AtomicBool::new(false);
pub mod utils;
pub use crate::utils::RenderQuality;
pub use eframe;
use eframe::egui::{self, Ui};
pub use na_seq;
#[cfg(not(target_arch = "wasm32"))]
pub use shader::{ImageBackground, ImageRenderer};

use eframe::egui::{Color32, Stroke, UserData, ViewportCommand};

use shader::Canvas;

use crate::scene::Animation;
pub use crate::utils::{Logger, RustLogger, Shape};
pub mod shapes;
use crate::scene::Scene;

type FeedbackSender = Publisher<ipc::Service, [u8], ()>;

pub mod scene;
fn feedback(tx: &FeedbackSender, message: Feedback) {
    let bytes = postcard::to_allocvec(&(VERSION, message)).unwrap();
    let mut sample = tx.loan_slice(bytes.len()).unwrap();
    sample.payload_mut().copy_from_slice(&bytes);
    assert_ne!(sample.send().unwrap(), 0, "feedback has no receiver");
}

fn decode_feedback(bytes: &[u8]) -> Feedback {
    let (version, message): (&str, Feedback) = postcard::from_bytes(bytes).unwrap();
    assert_eq!(version, VERSION, "feedback protocol mismatch");
    message
}
use image::{ImageBuffer, Rgba};

#[derive(serde::Serialize, serde::Deserialize)]
enum Request {
    InitializeScene {
        request_id: u64,
        scene: Scene,
        width: f32,
        height: f32,
    },
    InitializeAnimation {
        request_id: u64,
        animation: Animation,
        width: f32,
        height: f32,
    },
    UpdateScene {
        request_id: u64,
        scene: Scene,
    },
    TakeScreenshot {
        request_id: u64,
    },
}

fn decode_request(bytes: &[u8]) -> Request {
    let (version, request): (&str, Request) = postcard::from_bytes(bytes).unwrap();
    assert_eq!(version, VERSION, "request protocol mismatch");
    request
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Feedback {
    Ready {
        pid: u32,
    },
    /// App construction completed, not frame presentation.
    Initialized {
        request_id: u64,
    },
    Applied {
        request_id: u64,
    },
    Closed,
    ScreenshotTaken {
        request_id: u64,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
}
struct ReceiverState {
    rx: Subscriber<ipc::Service, [u8], ()>,
    tx: FeedbackSender,
}

fn create_ipc_services(
    name: &str,
) -> (
    Node<ipc::Service>,
    PortFactory<ipc::Service, [u8], ()>,
    PortFactory<ipc::Service, [u8], ()>,
) {
    iceoryx2::prelude::set_log_level(iceoryx2::prelude::LogLevel::Error);
    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();
    let scenes = node
        .service_builder(&format!("{name}/scene-v2").as_str().try_into().unwrap())
        .publish_subscribe::<[u8]>()
        .subscriber_max_buffer_size(4)
        .subscriber_max_borrowed_samples(2)
        .enable_safe_overflow(false)
        .open_or_create()
        .unwrap();
    let events = node
        .service_builder(&format!("{name}/feedback-v2").as_str().try_into().unwrap())
        .publish_subscribe::<[u8]>()
        .subscriber_max_buffer_size(16)
        .enable_safe_overflow(false)
        .open_or_create()
        .unwrap();
    (node, scenes, events)
}

pub fn register_render() {
    RENDER_REGISTERED.store(true, Ordering::Relaxed);

    let args: Vec<_> = std::env::args().collect();
    let child_index = args.iter().position(|a| a == "--child");
    if child_index.is_none() {
        return;
    }
    let name = match child_index {
        Some(i) => args
            .get(i + 1)
            .ok_or("missing service name")
            .unwrap()
            .clone(),
        None => format!(
            "cosmol-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ),
    };
    let headless = args.iter().any(|a| a == "--smoke" || a == "--headless");
    // run(
    //     child_index.is_some(),
    //     headless,
    //     headless || args.iter().any(|a| a == "--gui-smoke"),
    //     &name,
    // )
    let (_node, scenes, events) = create_ipc_services(&name);
    let rx = scenes.subscriber_builder().create().unwrap();
    let tx = events
        .publisher_builder()
        .initial_max_slice_len(64)
        .allocation_strategy(iceoryx2::prelude::AllocationStrategy::PowerOfTwo)
        .create()
        .unwrap();
    feedback(
        &tx,
        Feedback::Ready {
            pid: std::process::id(),
        },
    );
    let sample = loop {
        if let Some(sample) = rx.receive().unwrap() {
            break sample;
        }
        thread::sleep(Duration::from_millis(2));
    };
    let request = decode_request(sample.payload());
    let (width, height) = match &request {
        Request::InitializeScene { width, height, .. }
        | Request::InitializeAnimation { width, height, .. } => (*width, *height),
        Request::UpdateScene { .. } => panic!("expected initialization request"),
        Request::TakeScreenshot { .. } => panic!("expected initialization request"),
    };
    drop(sample);
    eframe::run_native(
        "COSMol Viewer iceoryx2",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([width, height]),
            depth_buffer: native_depth_buffer(),
            multisampling: native_multisampling(),
            renderer: eframe::Renderer::Glow,
            glow_options: native_glow_options(),
            ..Default::default()
        },
        Box::new(move |cc| {
            let (mut app, request_id) = match request {
                Request::InitializeScene {
                    request_id, scene, ..
                } => (App::new(cc, &scene, RustLogger), request_id),
                Request::InitializeAnimation {
                    request_id,
                    animation,
                    ..
                } => (App::new_play(cc, animation, RustLogger), request_id),
                Request::UpdateScene { .. } => unreachable!(),
                Request::TakeScreenshot { .. } => unreachable!(),
            };
            feedback(&tx, Feedback::Initialized { request_id });
            app.ipc = Some(ReceiverState { rx, tx });
            Ok(Box::new(app))
        }),
    )
    .unwrap();
    std::process::exit(0);
}

pub const BUILD_ID: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "_",
    compile_time::datetime_str!()
);

pub struct App<L: Logger> {
    ipc: Option<ReceiverState>,
    canvas: Canvas<L>,
    _gl: Option<Arc<eframe::glow::Context>>,
    pub ctx: egui::Context,
    screenshot_requested: Option<u64>,
    screenshot_result: Option<(Arc<egui::ColorImage>, egui::TextureHandle)>,
    _logger: L,
}

impl<L: Logger> App<L> {
    pub fn new(cc: &eframe::CreationContext<'_>, scene: &Scene, logger: L) -> Self {
        logger.log("Creating new viewer app...");
        let gl = cc.gl.clone();
        let canvas = Canvas::new(gl.as_ref().unwrap().clone(), scene, logger).unwrap();
        App {
            ipc: None,
            _gl: gl,
            canvas,
            ctx: cc.egui_ctx.clone(),
            screenshot_requested: None,
            screenshot_result: None,
            _logger: logger,
        }
    }

    pub fn new_play(cc: &eframe::CreationContext<'_>, animation: Animation, logger: L) -> Self {
        logger.log("Creating new viewer app...");
        let gl = cc.gl.clone();
        let canvas = Canvas::new_play(gl.as_ref().unwrap().clone(), animation, logger).unwrap();
        App {
            ipc: None,
            _gl: gl,
            canvas,
            ctx: cc.egui_ctx.clone(),
            screenshot_requested: None,
            screenshot_result: None,
            _logger: logger,
        }
    }

    pub fn update_scene(&mut self, scene: &Scene) {
        self.canvas.update_scene(scene);
    }

    pub fn set_camera_parameter_logging(&mut self, enabled: bool) {
        self.canvas.set_camera_parameter_logging(enabled);
    }

    // pub fn take_screenshot(&mut self) {
    //     self.screenshot_requested = true;
    // }

    pub fn poll_screenshot(&mut self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        if let Some((arc_image, _handle)) = self.screenshot_result.take() {
            let image = arc_image.as_ref();
            let width = image.size[0] as u32;
            let height = image.size[1] as u32;
            let raw_rgba = color_image_to_rgba_bytes(image);

            let buffer: ImageBuffer<Rgba<u8>, _> =
                ImageBuffer::from_raw(width, height, raw_rgba).expect("Invalid dimensions or data");

            Some(buffer)
        } else {
            None
        }
    }
}

fn color_image_to_rgba_bytes(image: &egui::ColorImage) -> Vec<u8> {
    image.pixels.iter().flat_map(|c| c.to_array()).collect()
}

impl<L: Logger> eframe::App for App<L> {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        if let Some(state) = &self.ipc {
            // if let Some((arc_image, _handle)) = self.screenshot_result.take() {
            //     let image = arc_image.as_ref();
            //     let width = image.size[0] as u32;
            //     let height = image.size[1] as u32;
            //     let raw_rgba = color_image_to_rgba_bytes(image);
            // }
            for _ in 0..4 {
                let Some(sample) = state.rx.receive().unwrap() else {
                    break;
                };
                match decode_request(sample.payload()) {
                    Request::UpdateScene { request_id, scene } => {
                        self.canvas.update_scene(&scene);
                        feedback(&state.tx, Feedback::Applied { request_id });
                    }
                    Request::TakeScreenshot { request_id } => {
                        self.screenshot_requested = Some(request_id);
                    }
                    _ => panic!("expected UpdateScene, received an initialization request"),
                }
            }
            ui.ctx().request_repaint_after(Duration::from_millis(5));
        }
        #[cfg(not(target_arch = "wasm32"))]
        egui_extras::install_image_loaders(ui);
        let panel_fill = if self.canvas.transparent_background() {
            Color32::TRANSPARENT
        } else {
            Color32::from_rgb(48, 48, 48)
        };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(panel_fill)
                    .inner_margin(0.0)
                    .outer_margin(0.0)
                    .stroke(Stroke::new(0.0, Color32::from_rgb(30, 200, 30))),
            )
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                self.canvas.custom_painting(ui);
                if let Some(request_id) = self.screenshot_requested.take() {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Screenshot(UserData::new(request_id)));
                }

                let screenshot = ui.ctx().input(|i| {
                    i.events
                        .iter()
                        .filter_map(|e| {
                            if let egui::Event::Screenshot {
                                image, user_data, ..
                            } = e
                            {
                                let request_id = *user_data.data.as_ref()?.downcast_ref::<u64>()?;
                                Some((request_id, image.clone()))
                            } else {
                                None
                            }
                        })
                        .next_back()
                });

                if let Some((request_id, image)) = screenshot {
                    if let Some(state) = &self.ipc {
                        feedback(
                            &state.tx,
                            Feedback::ScreenshotTaken {
                                request_id,
                                width: image.size[0] as u32,
                                height: image.size[1] as u32,
                                rgba: color_image_to_rgba_bytes(&image),
                            },
                        );
                    }
                }
            });
    }

    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        if let Some(state) = &self.ipc {
            feedback(&state.tx, Feedback::Closed);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.canvas.transparent_background() {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()
        }
    }
}

pub struct NativeGuiViewer {
    tx: Publisher<ipc::Service, [u8], ()>,
    rx: Subscriber<ipc::Service, [u8], ()>,
    request_id: Cell<u64>,
    closed: Cell<bool>,
    child: ChildGuard,
}

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Call cosmol_viewer_core::register_render() at the start of main()")]
    NotRegistered,
    #[error("No frames provided")]
    NoFramesProvided,
    #[error("Timeout waiting for App to initialize")]
    InitializationTimeout,
}

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("closed")]
    Closed,
    #[error("other: {0}")]
    Other(String),
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
const TIMEOUT: Duration = Duration::from_secs(30);

fn wait_event(rx: &Subscriber<ipc::Service, [u8], ()>, child: &mut ChildGuard) -> Feedback {
    let start = Instant::now();
    loop {
        if let Some(sample) = rx.receive().unwrap() {
            return decode_feedback(sample.payload());
        }
        if let Some(status) = child.0.try_wait().unwrap() {
            // panic!(format!("viewer exited before feedback: {status}"));
            panic!("viewer exited before feedback: {status}");
        }
        if start.elapsed() > TIMEOUT {
            panic!("feedback timeout");
        }
        thread::sleep(Duration::from_millis(2));
    }
}

impl NativeGuiViewer {
    pub fn new() -> Result<Self, RenderError> {
        if !RENDER_REGISTERED.load(Ordering::Relaxed) {
            return Err(RenderError::NotRegistered);
        }

        let name = format!(
            "cosmol-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let (_node, scenes, events) = create_ipc_services(&name);
        let tx = scenes
            .publisher_builder()
            .initial_max_slice_len(4096)
            .allocation_strategy(iceoryx2::prelude::AllocationStrategy::PowerOfTwo)
            .create()
            .unwrap();
        let rx = events.subscriber_builder().create().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--child", &name]).stdin(Stdio::null());
        let mut child = ChildGuard(command.spawn().unwrap());
        let pid = match wait_event(&rx, &mut child) {
            Feedback::Ready { pid } => pid,
            other => panic!("expected Ready, received {other:?}"),
        };
        eprintln!(
            "iceoryx2: producer PID {} -> viewer PID {}",
            std::process::id(),
            pid
        );
        Ok(Self {
            tx,
            rx,
            request_id: Cell::new(0),
            closed: Cell::new(false),
            child,
        })
    }

    pub fn render(scene: &Scene, width: f32, height: f32) -> Result<Self, RenderError> {
        Self::render_with_quality(scene, width, height, RenderQuality::Medium)
    }

    pub fn render_with_quality(
        scene: &Scene,
        width: f32,
        height: f32,
        quality: RenderQuality,
    ) -> Result<Self, RenderError> {
        let mut this = Self::new()?;
        let bytes = postcard::to_allocvec(&(
            VERSION,
            Request::InitializeScene {
                request_id: 0,
                scene: scene.clone(),
                width,
                height,
            },
        ))
        .unwrap();
        let mut sample = this.tx.loan_slice(bytes.len()).unwrap();
        sample.payload_mut().copy_from_slice(&bytes);
        assert_eq!(sample.send().unwrap(), 1);
        assert_eq!(
            wait_event(&this.rx, &mut this.child),
            Feedback::Initialized { request_id: 0 }
        );
        Ok(this)
    }

    pub fn is_open(&self) -> bool {
        !self.closed.get()
    }

    /// Keeps the viewer alive while blocking the calling thread for console input.
    ///
    /// Prints an exit prompt and waits until Enter is pressed or stdin reaches EOF.
    /// This consumes the viewer. On return, including an I/O error, its child
    /// process is closed and reaped by the existing cleanup guard.
    pub fn keep_alive(self) -> std::io::Result<()> {
        use std::io::{self, Write};

        println!("Press Enter to exit...");
        io::stdout().flush()?;
        io::stdin().read_line(&mut String::new())?;

        drop(self);
        Ok(())
    }

    pub fn update(&self, scene: &Scene) {
        if self.closed.get() {
            return;
        }
        while let Some(event) = self.rx.receive().unwrap() {
            if decode_feedback(event.payload()) == Feedback::Closed {
                self.closed.set(true);
                return;
            }
        }
        let request_id = self.request_id.get() + 1;
        self.request_id.set(request_id);
        let bytes = postcard::to_allocvec(&(
            VERSION,
            Request::UpdateScene {
                request_id,
                scene: scene.clone(),
            },
        ))
        .unwrap();
        let mut sample = self.tx.loan_slice(bytes.len()).unwrap();
        sample.payload_mut().copy_from_slice(&bytes);
        sample.send().unwrap();
        let start = Instant::now();
        loop {
            if let Some(event) = self.rx.receive().unwrap() {
                match decode_feedback(event.payload()) {
                    Feedback::Closed => {
                        self.closed.set(true);
                        return;
                    }
                    Feedback::Applied { request_id: ack } => assert_eq!(ack, request_id),
                    other => panic!("expected Applied, received {other:?}"),
                }
                break;
            }
            assert!(start.elapsed() < TIMEOUT, "update acknowledgment timeout");
            thread::sleep(Duration::from_millis(2));
        }
    }

    // pub fn set_camera_parameter_logging(&self, enabled: bool) {
    //     let mut app_guard = self.app.lock().unwrap();
    //     if let Some(app) = &mut *app_guard {
    //         app.set_camera_parameter_logging(enabled);
    //         app.ctx.request_repaint();
    //     } else {
    //         panic!("App not initialized")
    //     }
    // }

    pub fn take_screenshot(&self) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ImageError> {
        if self.closed.get() {
            return Err(ImageError::Other("closed".to_string()));
        }
        while let Some(event) = self.rx.receive().unwrap() {
            if decode_feedback(event.payload()) == Feedback::Closed {
                self.closed.set(true);
                return Err(ImageError::Closed);
            }
        }
        let request_id = self.request_id.get() + 1;
        self.request_id.set(request_id);
        let bytes =
            postcard::to_allocvec(&(VERSION, Request::TakeScreenshot { request_id })).unwrap();
        let mut sample = self.tx.loan_slice(bytes.len()).unwrap();
        sample.payload_mut().copy_from_slice(&bytes);
        sample.send().unwrap();

        loop {
            if let Some(event) = self.rx.receive().unwrap() {
                match decode_feedback(event.payload()) {
                    Feedback::ScreenshotTaken {
                        request_id: _,
                        width,
                        height,
                        rgba,
                    } => {
                        let buffer: ImageBuffer<Rgba<u8>, _> =
                            ImageBuffer::from_raw(width, height, rgba)
                                .expect("Invalid dimensions or data");
                        return Ok(buffer);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn play(animation: Animation, width: f32, height: f32) -> Result<Self, RenderError> {
        if animation.frames.is_empty() {
            return Err(RenderError::NoFramesProvided);
        }

        let mut this = Self::new()?;
        let bytes = postcard::to_allocvec(&(
            VERSION,
            Request::InitializeAnimation {
                request_id: 1,
                animation,
                width,
                height,
            },
        ))
        .unwrap();
        let mut sample = this.tx.loan_slice(bytes.len()).unwrap();
        sample.payload_mut().copy_from_slice(&bytes);
        assert_eq!(sample.send().unwrap(), 1);
        assert_eq!(
            wait_event(&this.rx, &mut this.child),
            Feedback::Initialized { request_id: 1 }
        );
        assert!(this.child.0.wait().unwrap().success());
        Ok(this)
    }

    pub fn save_video(
        animation: Animation,
        filename: &str,
        width: f32,
        height: f32,
        fps: Option<u32>, // If None, fps = 1 / animation.interval
    ) -> Result<Self, RenderError> {
        unimplemented!()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_multisampling() -> u16 {
    std::env::var("COSMOL_VIEWER_MULTISAMPLING")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4)
}

#[cfg(not(target_arch = "wasm32"))]
fn native_depth_buffer() -> u8 {
    std::env::var("COSMOL_VIEWER_DEPTH_BUFFER")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(24)
}

#[cfg(not(target_arch = "wasm32"))]
fn native_vsync() -> bool {
    std::env::var("COSMOL_VIEWER_VSYNC")
        .ok()
        .and_then(|value| match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(true)
}

#[cfg(not(target_arch = "wasm32"))]
fn native_glow_options() -> eframe::egui_glow::GlowConfiguration {
    eframe::egui_glow::GlowConfiguration {
        vsync: native_vsync(),
        ..Default::default()
    }
}

fn load_icon() -> IconData {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));
    let image = image::load_from_memory(bytes)
        .expect("Failed to load icon")
        .into_rgba8();

    let (width, height) = image.dimensions();

    IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
