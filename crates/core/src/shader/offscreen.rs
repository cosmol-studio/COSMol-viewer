use std::{ffi::CString, num::NonZeroU32, thread};

use eframe::glow::{self, HasContext as _};
use egui_winit::winit::{
    event_loop::EventLoop,
    raw_window_handle::{HasWindowHandle as _, RawWindowHandle},
    window::{Window, WindowAttributes},
};
use glam::Vec4;
use glutin::{
    config::{Config, ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder, GlProfile, Version},
    display::{GetGlDisplay as _, GlDisplay as _},
    prelude::*,
    surface::{PbufferSurface, Surface, SurfaceAttributesBuilder},
};
use glutin_winit::{ApiPreference, DisplayBuilder};
use image::{ImageBuffer, Rgba};

use crate::{Scene, shader::CameraState};

use super::canvas::Shader;

pub struct ImageRenderer;

#[derive(Clone, Copy, Debug)]
pub enum ImageBackground {
    Scene,
    Color([f32; 4]),
}

struct OffscreenGl {
    gl: glow::Context,
    _context: glutin::context::PossiblyCurrentContext,
    _surface: Surface<PbufferSurface>,
    _window: Option<Window>,
}

impl ImageRenderer {
    pub fn render(
        scene: &Scene,
        width: u32,
        height: u32,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
        Self::render_with_background(scene, width, height, ImageBackground::Scene)
    }

    pub fn render_with_background(
        scene: &Scene,
        width: u32,
        height: u32,
        background: ImageBackground,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
        let scene = scene.clone();
        thread::Builder::new()
            .name("cosmol_viewer_offscreen_render".to_owned())
            .spawn(move || {
                let mut gl = OffscreenGl::new()?;
                gl.render(&scene, width, height, background)
            })
            .map_err(|err| format!("failed to start offscreen render thread: {err}"))?
            .join()
            .map_err(|_| "offscreen render thread panicked".to_owned())?
    }

    pub fn save_png(
        scene: &Scene,
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        Self::save_png_with_background(scene, path, width, height, ImageBackground::Scene)
    }

    pub fn save_png_with_background(
        scene: &Scene,
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
        background: ImageBackground,
    ) -> Result<(), String> {
        let image = Self::render_with_background(scene, width, height, background)?;
        image.save(path).map_err(|err| err.to_string())
    }

    pub fn render_png_bytes(scene: &Scene, width: u32, height: u32) -> Result<Vec<u8>, String> {
        Self::render_png_bytes_with_background(scene, width, height, ImageBackground::Scene)
    }

    pub fn render_png_bytes_with_background(
        scene: &Scene,
        width: u32,
        height: u32,
        background: ImageBackground,
    ) -> Result<Vec<u8>, String> {
        let image = Self::render_with_background(scene, width, height, background)?;
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .map_err(|err| err.to_string())?;
        Ok(bytes)
    }
}

impl OffscreenGl {
    fn new() -> Result<Self, String> {
        let width = NonZeroU32::new(1).expect("1 is non-zero");
        let height = NonZeroU32::new(1).expect("1 is non-zero");

        #[cfg(target_os = "linux")]
        if linux_is_displayless() {
            return Self::new_headless_egl(width, height);
        }

        Self::new_with_winit(width, height)
    }

    fn new_with_winit(width: NonZeroU32, height: NonZeroU32) -> Result<Self, String> {
        let event_loop = offscreen_event_loop_builder().build().map_err(|err| {
            format!("{err}. Offscreen rendering could not create its GL bootstrap event loop.")
        })?;
        let template = offscreen_config_template_builder(width, height);

        let (window, gl_config) = DisplayBuilder::new()
            .with_preference(ApiPreference::FallbackEgl)
            .with_window_attributes(bootstrap_window_attributes())
            .build(&event_loop, template, |configs| {
                configs
                    .max_by_key(|config| config.num_samples())
                    .expect("no GL configs found")
            })
            .map_err(|err| err.to_string())?;

        let raw_window_handle = window
            .as_ref()
            .and_then(|window| window.window_handle().ok())
            .map(|handle| handle.as_raw());

        Self::new_from_config(width, height, gl_config, raw_window_handle, window)
    }

    #[cfg(target_os = "linux")]
    fn new_headless_egl(width: NonZeroU32, height: NonZeroU32) -> Result<Self, String> {
        use egui_winit::winit::raw_window_handle::{RawDisplayHandle, XlibDisplayHandle};
        use glutin::{
            api::egl::{device::Device, display::Display as EglDisplay},
            display::{Display, DisplayApiPreference},
        };

        let mut errors = Vec::new();

        match Device::query_devices() {
            Ok(devices) => {
                for device in devices {
                    let egl_display = match unsafe { EglDisplay::with_device(&device, None) } {
                        Ok(display) => display,
                        Err(err) => {
                            errors.push(format!("EGL device display: {err}"));
                            continue;
                        }
                    };
                    let gl_display = Display::Egl(egl_display);
                    match Self::new_from_headless_display(
                        width,
                        height,
                        gl_display,
                        "EGL device display",
                    ) {
                        Ok(gl) => return Ok(gl),
                        Err(err) => errors.push(err),
                    }
                }
            }
            Err(err) => errors.push(format!("{err}. Headless EGL device enumeration failed.")),
        }

        let raw_display = RawDisplayHandle::Xlib(XlibDisplayHandle::new(None, 0));
        match unsafe { Display::new(raw_display, DisplayApiPreference::Egl) } {
            Ok(gl_display) => {
                match Self::new_from_headless_display(
                    width,
                    height,
                    gl_display,
                    "EGL default display",
                ) {
                    Ok(gl) => return Ok(gl),
                    Err(err) => errors.push(err),
                }
            }
            Err(err) => errors.push(format!("EGL default display: {err}")),
        }

        Err(format!(
            "Headless EGL could not create an offscreen display{}",
            if errors.is_empty() {
                ".".to_owned()
            } else {
                format!(": {}", errors.join("; "))
            }
        ))
    }

    #[cfg(target_os = "linux")]
    fn new_from_headless_display(
        width: NonZeroU32,
        height: NonZeroU32,
        gl_display: glutin::display::Display,
        label: &str,
    ) -> Result<Self, String> {
        let template = offscreen_config_template_builder(width, height).build();
        let gl_config = match unsafe { gl_display.find_configs(template) } {
            Ok(configs) => configs.max_by_key(|config| config.num_samples()),
            Err(err) => return Err(format!("{label}: {err}")),
        };

        match gl_config {
            Some(gl_config) => Self::new_from_config(width, height, gl_config, None, None)
                .map_err(|err| format!("{label}: {err}")),
            None => Err(format!("{label}: no GL configs found")),
        }
    }

    fn new_from_config(
        width: NonZeroU32,
        height: NonZeroU32,
        gl_config: Config,
        raw_window_handle: Option<RawWindowHandle>,
        window: Option<Window>,
    ) -> Result<Self, String> {
        let gl_display = gl_config.display();

        let context_attributes = ContextAttributesBuilder::new()
            .with_profile(GlProfile::Core)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(raw_window_handle);

        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(raw_window_handle);

        let not_current = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .or_else(|_| gl_display.create_context(&gl_config, &fallback_context_attributes))
                .map_err(|err| err.to_string())?
        };

        let surface_attributes = SurfaceAttributesBuilder::<PbufferSurface>::new()
            .with_single_buffer(true)
            .build(width, height);
        let surface = unsafe {
            gl_display
                .create_pbuffer_surface(&gl_config, &surface_attributes)
                .map_err(|err| err.to_string())?
        };
        let context = not_current
            .make_current(&surface)
            .map_err(|err| err.to_string())?;

        let gl = unsafe {
            glow::Context::from_loader_function(|symbol| {
                let symbol = CString::new(symbol).expect("GL symbol contained NUL");
                gl_display.get_proc_address(&symbol)
            })
        };

        Ok(Self {
            gl,
            _context: context,
            _surface: surface,
            _window: window,
        })
    }

    fn render(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        background: ImageBackground,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
        if width == 0 || height == 0 {
            return Err("width and height must be non-zero".to_owned());
        }

        let gl = &self.gl;
        let mut shader =
            Shader::new(gl, scene).ok_or_else(|| "failed to initialize shader".to_owned())?;
        if let ImageBackground::Color(background_color) = background {
            shader.set_background_color(Vec4::from_array(background_color));
        }
        let camera_state = scene.camera_state.unwrap_or_else(CameraState::default);
        let aspect_ratio = width as f32 / height as f32;
        let samples = offscreen_samples(gl);

        unsafe {
            let msaa_framebuffer = gl.create_framebuffer().map_err(|err| err.to_string())?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(msaa_framebuffer));

            let msaa_color = gl.create_renderbuffer().map_err(|err| err.to_string())?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(msaa_color));
            gl.renderbuffer_storage_multisample(
                glow::RENDERBUFFER,
                samples,
                glow::RGBA8,
                width as i32,
                height as i32,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(msaa_color),
            );

            let msaa_depth = gl.create_renderbuffer().map_err(|err| err.to_string())?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(msaa_depth));
            gl.renderbuffer_storage_multisample(
                glow::RENDERBUFFER,
                samples,
                glow::DEPTH_COMPONENT24,
                width as i32,
                height as i32,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(msaa_depth),
            );

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                gl.delete_renderbuffer(msaa_depth);
                gl.delete_renderbuffer(msaa_color);
                gl.delete_framebuffer(msaa_framebuffer);
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                return Err(format!(
                    "offscreen MSAA framebuffer is incomplete: 0x{status:x}"
                ));
            }

            gl.viewport(0, 0, width as i32, height as i32);
            shader.paint(gl, aspect_ratio, &camera_state);
            gl.finish();

            let resolve_framebuffer = gl.create_framebuffer().map_err(|err| err.to_string())?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resolve_framebuffer));

            let resolve_texture = gl.create_texture().map_err(|err| err.to_string())?;
            gl.bind_texture(glow::TEXTURE_2D, Some(resolve_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(resolve_texture),
                0,
            );

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                gl.delete_texture(resolve_texture);
                gl.delete_framebuffer(resolve_framebuffer);
                gl.delete_renderbuffer(msaa_depth);
                gl.delete_renderbuffer(msaa_color);
                gl.delete_framebuffer(msaa_framebuffer);
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                return Err(format!(
                    "offscreen resolve framebuffer is incomplete: 0x{status:x}"
                ));
            }

            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(msaa_framebuffer));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(resolve_framebuffer));
            gl.blit_framebuffer(
                0,
                0,
                width as i32,
                height as i32,
                0,
                0,
                width as i32,
                height as i32,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resolve_framebuffer));
            let mut pixels = vec![0_u8; width as usize * height as usize * 4];
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_texture(resolve_texture);
            gl.delete_framebuffer(resolve_framebuffer);
            gl.delete_renderbuffer(msaa_depth);
            gl.delete_renderbuffer(msaa_color);
            gl.delete_framebuffer(msaa_framebuffer);

            flip_rgba_rows(&mut pixels, width as usize, height as usize);

            ImageBuffer::from_raw(width, height, pixels)
                .ok_or_else(|| "failed to build image buffer from GL pixels".to_owned())
        }
    }
}

fn offscreen_samples(gl: &glow::Context) -> i32 {
    let requested = std::env::var("COSMOL_VIEWER_OFFSCREEN_SAMPLES")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(4)
        .clamp(1, 16);

    unsafe {
        let max_samples = gl.get_parameter_i32(glow::MAX_SAMPLES).max(1);
        requested.min(max_samples)
    }
}

fn offscreen_config_template_builder(
    width: NonZeroU32,
    height: NonZeroU32,
) -> ConfigTemplateBuilder {
    ConfigTemplateBuilder::new()
        .with_depth_size(24)
        .with_alpha_size(8)
        .with_pbuffer_sizes(width, height)
}

fn offscreen_event_loop_builder() -> egui_winit::winit::event_loop::EventLoopBuilder<()> {
    let mut builder = EventLoop::builder();
    #[cfg(target_family = "windows")]
    {
        use egui_winit::winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    #[cfg(feature = "wayland")]
    {
        use egui_winit::winit::platform::wayland::EventLoopBuilderExtWayland;
        builder.with_any_thread(true);
    }
    #[cfg(feature = "x11")]
    {
        use egui_winit::winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_any_thread(true);
    }
    builder
}

#[cfg(target_os = "linux")]
fn linux_is_displayless() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var_os("WAYLAND_SOCKET").is_none()
        && std::env::var_os("DISPLAY").is_none()
}

fn bootstrap_window_attributes() -> Option<WindowAttributes> {
    if cfg!(target_os = "windows") {
        Some(
            WindowAttributes::default()
                .with_visible(false)
                .with_title("cosmol_viewer_offscreen_context"),
        )
    } else {
        None
    }
}

fn flip_rgba_rows(pixels: &mut [u8], width: usize, height: usize) {
    let stride = width * 4;
    for y in 0..(height / 2) {
        let top = y * stride;
        let bottom = (height - 1 - y) * stride;
        for x in 0..stride {
            pixels.swap(top + x, bottom + x);
        }
    }
}
