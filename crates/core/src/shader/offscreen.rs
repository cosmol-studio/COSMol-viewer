use eframe::glow::{self, HasContext as _};
use glam::Vec4;
use image::{ImageBuffer, Rgba};
use surfman::{
    Connection, ContextAttributeFlags, ContextAttributes, GLApi, GLVersion, SurfaceAccess,
    SurfaceType,
};

use crate::{Scene, shader::CameraState};

use super::canvas::Shader;

surfman::declare_surfman!();

/// Renders images synchronously on the calling thread, without a viewer window.
///
/// This creates a separate GL context. Callers that already have a GL context
/// current on this thread must make their own context current again afterwards.
pub struct ImageRenderer;

#[derive(Clone, Copy, Debug)]
pub enum ImageBackground {
    Scene,
    Color([f32; 4]),
}

struct OffscreenGl {
    gl: glow::Context,
    _context: OffscreenContext,
}

// Surfman requires explicit destruction, including when initialization returns early.
// Keep the context and its device together so every error path releases both.
struct OffscreenContext {
    device: surfman::Device,
    context: surfman::Context,
}

impl Drop for OffscreenContext {
    fn drop(&mut self) {
        if let Err(err) = self.device.destroy_context(&mut self.context) {
            eprintln!("[WARN] Failed to destroy offscreen GL context: {err:?}");
        }
    }
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
        if width == 0 || height == 0 {
            return Err("width and height must be non-zero".to_owned());
        }
        offscreen_trace("creating offscreen GL backend");
        let mut gl = OffscreenGl::new()?;
        offscreen_trace("created offscreen GL backend");
        let image = gl.render(scene, width, height, background)?;
        offscreen_trace("completed offscreen render");
        Ok(image)
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
        let connection = Connection::new()
            .map_err(|err| format!("failed to open offscreen GL connection: {err:?}"))?;
        let adapter = if software_gl_requested() {
            connection.create_software_adapter()
        } else {
            connection.create_adapter()
        }
        .map_err(|err| format!("failed to select offscreen GL adapter: {err:?}"))?;
        let device = connection
            .create_device(&adapter)
            .map_err(|err| format!("failed to create offscreen GL device: {err:?}"))?;
        let attributes = ContextAttributes {
            version: match device.gl_api() {
                GLApi::GL => GLVersion::new(3, 3),
                GLApi::GLES => GLVersion::new(3, 0),
            },
            flags: ContextAttributeFlags::ALPHA | ContextAttributeFlags::DEPTH,
        };
        let descriptor = device
            .create_context_descriptor(&attributes)
            .map_err(|err| format!("failed to describe offscreen GL context: {err:?}"))?;
        let context = device
            .create_context(&descriptor, None)
            .map_err(|err| format!("failed to create offscreen GL context: {err:?}"))?;
        let mut context = OffscreenContext { device, context };
        // Rendering uses our own FBOs; the small generic surface only bootstraps GL.
        let surface = context
            .device
            .create_surface(
                &context.context,
                SurfaceAccess::GPUOnly,
                SurfaceType::Generic {
                    size: (1, 1).into(),
                },
            )
            .map_err(|err| format!("failed to create offscreen GL surface: {err:?}"))?;
        if let Err((err, mut surface)) = context
            .device
            .bind_surface_to_context(&mut context.context, surface)
        {
            let _ = context
                .device
                .destroy_surface(&mut context.context, &mut surface);
            return Err(format!("failed to bind offscreen GL surface: {err:?}"));
        }
        context
            .device
            .make_context_current(&context.context)
            .map_err(|err| format!("failed to activate offscreen GL context: {err:?}"))?;
        let gl = unsafe {
            glow::Context::from_loader_function(|symbol| {
                context.device.get_proc_address(&context.context, symbol)
            })
        };
        Ok(Self {
            gl,
            _context: context,
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
        offscreen_trace("creating scene shader");
        let mut shader =
            Shader::new(gl, scene).ok_or_else(|| "failed to initialize shader".to_owned())?;
        offscreen_trace("created scene shader");
        if let ImageBackground::Color(background_color) = background {
            shader.set_background_color(Vec4::from_array(background_color));
        }
        let camera_state = scene.camera_state.unwrap_or_else(CameraState::default);
        let aspect_ratio = width as f32 / height as f32;
        let samples = offscreen_samples(gl);

        unsafe {
            if samples == 1 {
                return render_single_sample(
                    gl,
                    &mut shader,
                    &camera_state,
                    aspect_ratio,
                    width,
                    height,
                );
            }

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
            shader.paint(gl, aspect_ratio, &camera_state, true);
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

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn render_single_sample(
    gl: &glow::Context,
    shader: &mut Shader,
    camera_state: &CameraState,
    aspect_ratio: f32,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    offscreen_trace("creating single-sample framebuffer");
    let framebuffer = gl.create_framebuffer().map_err(|err| err.to_string())?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

    let color = gl.create_texture().map_err(|err| err.to_string())?;
    gl.bind_texture(glow::TEXTURE_2D, Some(color));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA8 as i32,
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
        Some(color),
        0,
    );

    let depth = gl.create_renderbuffer().map_err(|err| err.to_string())?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
    gl.renderbuffer_storage(
        glow::RENDERBUFFER,
        glow::DEPTH_COMPONENT24,
        width as i32,
        height as i32,
    );
    gl.framebuffer_renderbuffer(
        glow::FRAMEBUFFER,
        glow::DEPTH_ATTACHMENT,
        glow::RENDERBUFFER,
        Some(depth),
    );

    let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
    if status != glow::FRAMEBUFFER_COMPLETE {
        gl.delete_renderbuffer(depth);
        gl.delete_texture(color);
        gl.delete_framebuffer(framebuffer);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        return Err(format!(
            "offscreen single-sample framebuffer is incomplete: 0x{status:x}"
        ));
    }

    offscreen_trace("painting single-sample framebuffer");
    gl.viewport(0, 0, width as i32, height as i32);
    shader.paint(gl, aspect_ratio, camera_state, false);
    offscreen_trace("finishing single-sample framebuffer");
    gl.finish();

    offscreen_trace("reading single-sample framebuffer");
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

    offscreen_trace("releasing single-sample framebuffer");
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    gl.delete_renderbuffer(depth);
    gl.delete_texture(color);
    gl.delete_framebuffer(framebuffer);

    flip_rgba_rows(&mut pixels, width as usize, height as usize);
    ImageBuffer::from_raw(width, height, pixels)
        .ok_or_else(|| "failed to build image buffer from GL pixels".to_owned())
}

fn offscreen_trace(stage: &str) {
    if std::env::var_os("COSMOL_VIEWER_OFFSCREEN_TRACE").is_none() {
        return;
    }

    use std::io::Write as _;
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "[cosmol_viewer offscreen] {stage}");
    let _ = stderr.flush();
}

fn offscreen_samples(gl: &glow::Context) -> i32 {
    let requested = std::env::var("COSMOL_VIEWER_OFFSCREEN_SAMPLES")
        .ok()
        .and_then(|value| value.parse::<i32>().ok());

    let default_samples = if software_gl_requested() || software_gl_renderer(gl) {
        1
    } else {
        4
    };
    let requested = requested.unwrap_or(default_samples).clamp(1, 16);

    unsafe {
        let max_samples = gl.get_parameter_i32(glow::MAX_SAMPLES).max(1);
        requested.min(max_samples)
    }
}

fn software_gl_requested() -> bool {
    std::env::var("LIBGL_ALWAYS_SOFTWARE")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn software_gl_renderer(gl: &glow::Context) -> bool {
    let renderer = unsafe { gl.get_parameter_string(glow::RENDERER) }.to_ascii_lowercase();
    renderer.contains("llvmpipe")
        || renderer.contains("softpipe")
        || renderer.contains("swrast")
        || renderer.contains("software rasterizer")
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
