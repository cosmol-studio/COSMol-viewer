mod shader;
use crate::egui::IconData;
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub mod parser;
pub mod utils;
pub use crate::utils::RenderQuality;
pub use eframe;
use eframe::egui::{self, Ui};
pub use na_seq;

use eframe::egui::{Color32, Stroke, UserData, ViewportCommand};

use shader::Canvas;

use crate::scene::Animation;
pub use crate::utils::{Logger, RustLogger, Shape};
pub mod shapes;
use crate::scene::Scene;

pub mod scene;
use image::{ImageBuffer, Rgba};

pub struct AppWrapper<L: Logger>(pub Arc<Mutex<Option<App<L>>>>);

pub const BUILD_ID: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "_",
    compile_time::datetime_str!()
);

impl<L: Logger> eframe::App for AppWrapper<L> {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if let Some(app) = &mut *self.0.lock().unwrap() {
            app.ui(ui, frame);
        }
    }
}

pub struct App<L: Logger> {
    canvas: Canvas<L>,
    _gl: Option<Arc<eframe::glow::Context>>,
    pub ctx: egui::Context,
    screenshot_requested: bool,
    screenshot_result: Option<(Arc<egui::ColorImage>, egui::TextureHandle)>,
    _logger: L,
}

impl<L: Logger> App<L> {
    pub fn new(cc: &eframe::CreationContext<'_>, scene: &Scene, logger: L) -> Self {
        logger.log("Creating new viewer app...");
        let gl = cc.gl.clone();
        let canvas = Canvas::new(gl.as_ref().unwrap().clone(), scene, logger).unwrap();
        App {
            _gl: gl,
            canvas,
            ctx: cc.egui_ctx.clone(),
            screenshot_requested: false,
            screenshot_result: None,
            _logger: logger,
        }
    }

    pub fn new_play(cc: &eframe::CreationContext<'_>, animation: Animation, logger: L) -> Self {
        logger.log("Creating new viewer app...");
        let gl = cc.gl.clone();
        let canvas = Canvas::new_play(gl.as_ref().unwrap().clone(), animation, logger).unwrap();
        App {
            _gl: gl,
            canvas,
            ctx: cc.egui_ctx.clone(),
            screenshot_requested: false,
            screenshot_result: None,
            _logger: logger,
        }
    }

    pub fn update_scene(&mut self, scene: &Scene) {
        self.canvas.update_scene(scene);
    }

    pub fn take_screenshot(&mut self) {
        self.screenshot_requested = true;
    }

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
        #[cfg(not(target_arch = "wasm32"))]
        egui_extras::install_image_loaders(ui);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(48, 48, 48))
                    .inner_margin(0.0)
                    .outer_margin(0.0)
                    .stroke(Stroke::new(0.0, Color32::from_rgb(30, 200, 30))),
            )
            .show_inside(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                self.canvas.custom_painting(ui);
                if self.screenshot_requested {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
                    self.screenshot_requested = false; // only request once
                }

                let image = ui.ctx().input(|i| {
                    i.events
                        .iter()
                        .filter_map(|e| {
                            if let egui::Event::Screenshot { image, .. } = e {
                                Some(image.clone())
                            } else {
                                None
                            }
                        })
                        .next_back()
                });

                if let Some(image) = image {
                    self.screenshot_result = Some((
                        image.clone(),
                        ui.ctx()
                            .load_texture("screenshot_demo", image, Default::default()),
                    ));
                }
            });
    }
}

pub struct NativeGuiViewer {
    pub app: Arc<Mutex<Option<App<RustLogger>>>>,
}

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("No frames provided")]
    NoFramesProvided,
    #[error("Timeout waiting for App to initialize")]
    InitializationTimeout,
}

impl NativeGuiViewer {
    pub fn render(scene: &Scene, width: f32, height: f32) -> Result<Self, RenderError> {
        Self::render_with_quality(scene, width, height, RenderQuality::Medium)
    }

    pub fn render_with_quality(
        scene: &Scene,
        width: f32,
        height: f32,
        quality: RenderQuality,
    ) -> Result<Self, RenderError> {
        use std::time::Duration;
        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        #[cfg(not(target_arch = "wasm32"))]
        use eframe::{
            NativeOptions,
            egui::{Vec2, ViewportBuilder},
        };

        let app: Arc<Mutex<Option<App<RustLogger>>>> = Arc::new(Mutex::new(None));
        let (tx, rx) = std::sync::mpsc::channel();
        let app_clone = Arc::clone(&app);

        let scene = Arc::new(scene.clone());
        #[cfg(not(target_arch = "wasm32"))]
        thread::spawn(move || {
            use eframe::{EventLoopBuilderHook, run_native};
            use std::process;
            let event_loop_builder: Option<EventLoopBuilderHook> =
                Some(Box::new(|event_loop_builder| {
                    #[cfg(target_family = "windows")]
                    {
                        use egui_winit::winit::platform::windows::EventLoopBuilderExtWindows;
                        event_loop_builder.with_any_thread(true);
                    }
                    #[cfg(feature = "wayland")]
                    {
                        use egui_winit::winit::platform::wayland::EventLoopBuilderExtWayland;
                        event_loop_builder.with_any_thread(true);
                    }
                    #[cfg(feature = "x11")]
                    {
                        use egui_winit::winit::platform::x11::EventLoopBuilderExtX11;
                        event_loop_builder.with_any_thread(true);
                    }
                }));

            let icon = load_icon();

            let native_options = NativeOptions {
                viewport: ViewportBuilder::default()
                    .with_inner_size(Vec2::new(width, height))
                    .with_icon(icon),
                depth_buffer: 24,
                multisampling: 4,
                event_loop_builder,
                ..Default::default()
            };

            let _ = run_native(
                "cosmol_viewer",
                native_options,
                Box::new(move |cc| {
                    let mut guard = app_clone.lock().unwrap();
                    *guard = Some(App::new(cc, scene.as_ref(), RustLogger));
                    let _ = tx.send(());
                    Ok(Box::new(AppWrapper(app_clone.clone())))
                }),
            );
            process::exit(0);
        });

        rx.recv_timeout(Duration::from_secs(30))
            .map_err(|_| RenderError::InitializationTimeout)?;

        Ok(Self { app })
    }

    pub fn update(&self, scene: &Scene) {
        let mut app_guard = self.app.lock().unwrap();
        if let Some(app) = &mut *app_guard {
            app.update_scene(scene);
            app.ctx.request_repaint();
        } else {
            panic!("App not initialized")
        }
    }

    pub fn take_screenshot(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
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
                    return image;
                }
            }
            drop(app_guard);
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    pub fn play(animation: Animation, width: f32, height: f32) -> Result<Self, RenderError> {
        if animation.frames.is_empty() {
            return Err(RenderError::NoFramesProvided);
        }

        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        #[cfg(not(target_arch = "wasm32"))]
        use eframe::{
            NativeOptions,
            egui::{Vec2, ViewportBuilder},
        };

        let app: Arc<Mutex<Option<App<RustLogger>>>> = Arc::new(Mutex::new(None));
        let app_clone = Arc::clone(&app);

        #[cfg(not(target_arch = "wasm32"))]
        thread::spawn(move || {
            use std::process;

            use eframe::{EventLoopBuilderHook, run_native};
            let event_loop_builder: Option<EventLoopBuilderHook> =
                Some(Box::new(|event_loop_builder| {
                    #[cfg(target_family = "windows")]
                    {
                        use egui_winit::winit::platform::windows::EventLoopBuilderExtWindows;
                        event_loop_builder.with_any_thread(true);
                    }
                    #[cfg(feature = "wayland")]
                    {
                        use egui_winit::winit::platform::wayland::EventLoopBuilderExtWayland;
                        event_loop_builder.with_any_thread(true);
                    }
                    #[cfg(feature = "x11")]
                    {
                        use egui_winit::winit::platform::x11::EventLoopBuilderExtX11;
                        event_loop_builder.with_any_thread(true);
                    }
                }));

            let native_options = NativeOptions {
                viewport: ViewportBuilder::default().with_inner_size(Vec2::new(width, height)),
                depth_buffer: 24,
                multisampling: 4,
                event_loop_builder,
                ..Default::default()
            };

            let _ = run_native(
                "cosmol_viewer",
                native_options,
                Box::new(move |cc| {
                    let mut guard = app_clone.lock().unwrap();
                    *guard = Some(App::new_play(cc, animation, RustLogger));
                    Ok(Box::new(AppWrapper(app_clone.clone())))
                }),
            );
            process::exit(0);
        });

        loop {}
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
