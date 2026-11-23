use eframe::{egui_glow, glow, glow::HasContext as _};
use glam::Mat4;

use super::canvas::Shader;
use crate::scene::Silhouettes;

struct Framebuffer {
    handle: glow::Framebuffer,
    color: glow::Texture,
    depth: glow::Texture,
    styles: Option<glow::Texture>,
    size: [i32; 2],
}

#[allow(unsafe_op_in_unsafe_fn)]
impl Framebuffer {
    unsafe fn new(gl: &glow::Context, size: [i32; 2], scoped: bool) -> Result<Self, String> {
        let handle = gl.create_framebuffer()?;
        let color = gl
            .create_texture()
            .inspect_err(|_| gl.delete_framebuffer(handle))?;
        let depth = gl.create_texture().inspect_err(|_| {
            gl.delete_texture(color);
            gl.delete_framebuffer(handle);
        })?;
        let styles = if scoped {
            Some(gl.create_texture().inspect_err(|_| {
                gl.delete_texture(depth);
                gl.delete_texture(color);
                gl.delete_framebuffer(handle);
            })?)
        } else {
            None
        };
        let target = Self {
            handle,
            color,
            depth,
            styles,
            size,
        };
        gl.active_texture(glow::TEXTURE0);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(handle));
        for (texture, internal_format, format, kind, attachment) in [
            Some((
                color,
                glow::RGBA8,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::COLOR_ATTACHMENT0,
            )),
            Some((
                depth,
                glow::DEPTH_COMPONENT24,
                glow::DEPTH_COMPONENT,
                glow::UNSIGNED_INT,
                glow::DEPTH_ATTACHMENT,
            )),
            styles.map(|texture| {
                (
                    texture,
                    glow::R32UI,
                    glow::RED_INTEGER,
                    glow::UNSIGNED_INT,
                    glow::COLOR_ATTACHMENT1,
                )
            }),
        ]
        .into_iter()
        .flatten()
        {
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal_format as i32,
                size[0],
                size[1],
                0,
                format,
                kind,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                attachment,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
        }
        gl.bind_texture(glow::TEXTURE_2D, None);

        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        if status != glow::FRAMEBUFFER_COMPLETE {
            target.delete(gl);
            return Err(format!(
                "silhouette framebuffer is incomplete: 0x{status:x}"
            ));
        }
        Ok(target)
    }

    unsafe fn delete(self, gl: &glow::Context) {
        gl.delete_framebuffer(self.handle);
        gl.delete_texture(self.color);
        gl.delete_texture(self.depth);
        if let Some(styles) = self.styles {
            gl.delete_texture(styles);
        }
    }
}

struct StylePalette {
    texture: glow::Texture,
    styles: Vec<Silhouettes>,
    columns: i32,
    max_width: f32,
}

impl StylePalette {
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn new(gl: &glow::Context, styles: &[Silhouettes]) -> Result<Self, String> {
        // Each style occupies two RGBA texels: color/width, then depth threshold.
        let max_size = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) as usize;
        let columns = styles.len().min(max_size / 2);
        let rows = styles.len().div_ceil(columns);
        if rows > max_size {
            return Err("Too many silhouette styles for the renderer".into());
        }
        let mut pixels = vec![[0.0; 4]; rows * columns * 2];
        for (style, pixel) in styles.iter().zip(pixels.chunks_exact_mut(2)) {
            pixel[0] = style.color.extend(style.width).to_array();
            pixel[1][0] = style.depth_jump;
        }
        let texture = gl.create_texture()?;
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA32F as i32,
            (columns * 2) as i32,
            rows as i32,
            0,
            glow::RGBA,
            glow::FLOAT,
            glow::PixelUnpackData::Slice(Some(bytemuck::cast_slice(&pixels))),
        );
        for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::NEAREST as i32);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(Self {
            texture,
            styles: styles.to_vec(),
            columns: columns as i32,
            max_width: styles.iter().map(|style| style.width).fold(0.0, f32::max),
        })
    }
}

pub(super) struct SilhouettePass {
    program: glow::Program,
    scoped_program: Option<glow::Program>,
    palette: Option<StylePalette>,
    target: Option<Framebuffer>,
}

pub(super) struct Destination {
    pub(super) framebuffer: Option<glow::Framebuffer>,
    pub(super) viewport: [i32; 4],
    pub(super) scissor_enabled: bool,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl Destination {
    pub(super) unsafe fn restore(&self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
        let [x, y, width, height] = self.viewport;
        gl.viewport(x, y, width, height);
        if self.scissor_enabled {
            gl.enable(glow::SCISSOR_TEST);
        } else {
            gl.disable(glow::SCISSOR_TEST);
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
impl SilhouettePass {
    pub(super) unsafe fn new(gl: &glow::Context) -> Self {
        Self {
            program: Shader::compile_program(
                gl,
                egui_glow::ShaderVersion::get(gl),
                "scene silhouettes",
                include_str!("bg_vertex.glsl"),
                include_str!("silhouette.glsl"),
            ),
            scoped_program: None,
            palette: None,
            target: None,
        }
    }

    pub(super) unsafe fn begin(
        &mut self,
        gl: &glow::Context,
        destination: &Destination,
        styles: &[Silhouettes],
    ) -> Result<bool, String> {
        let [_, _, width, height] = destination.viewport;
        if width <= 0 || height <= 0 {
            return Ok(false);
        }

        let scoped = !styles.is_empty();
        if scoped
            && self
                .palette
                .as_ref()
                .is_none_or(|palette| palette.styles != styles)
        {
            let palette = StylePalette::new(gl, styles)?;
            if let Some(previous) = self.palette.replace(palette) {
                gl.delete_texture(previous.texture);
            }
            self.scoped_program.get_or_insert_with(|| {
                Shader::compile_program(
                    gl,
                    egui_glow::ShaderVersion::get(gl),
                    "scoped silhouettes",
                    include_str!("bg_vertex.glsl"),
                    concat!(
                        "#define USE_SCOPED_SILHOUETTES\n",
                        include_str!("silhouette.glsl")
                    ),
                )
            });
        }
        if !scoped && let Some(palette) = self.palette.take() {
            gl.delete_texture(palette.texture);
        }

        let size = [width, height];
        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size != size || target.styles.is_some() != scoped)
        {
            let new_target = match Framebuffer::new(gl, size, scoped) {
                Ok(target) => target,
                Err(error) => {
                    destination.restore(gl);
                    return Err(error);
                }
            };
            self.release_target(gl);
            self.target = Some(new_target);
        }
        gl.bind_framebuffer(
            glow::FRAMEBUFFER,
            Some(self.target.as_ref().unwrap().handle),
        );
        gl.viewport(0, 0, width, height);
        // Egui's clip rectangle is in destination pixels. Apply it when compositing.
        gl.disable(glow::SCISSOR_TEST);
        if scoped {
            gl.color_mask(true, true, true, true);
            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0, glow::COLOR_ATTACHMENT1]);
            gl.clear_buffer_u32_slice(glow::COLOR, 1, &[0; 4]);
            self.use_color_only(gl);
        }
        Ok(true)
    }

    pub(super) unsafe fn write_style_tags(&self, gl: &glow::Context, depth_only: bool) -> bool {
        if self
            .target
            .as_ref()
            .is_some_and(|target| target.styles.is_some())
        {
            let color = if depth_only {
                glow::NONE
            } else {
                glow::COLOR_ATTACHMENT0
            };
            gl.draw_buffers(&[color, glow::COLOR_ATTACHMENT1]);
            true
        } else {
            false
        }
    }

    pub(super) unsafe fn use_color_only(&self, gl: &glow::Context) {
        if self
            .target
            .as_ref()
            .is_some_and(|target| target.styles.is_some())
        {
            gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
        }
    }

    pub(super) unsafe fn draw_edges(
        &self,
        gl: &glow::Context,
        vao: glow::VertexArray,
        settings: Option<Silhouettes>,
        projection: Mat4,
        scene_depth: f32,
    ) {
        let target = self.target.as_ref().unwrap();
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::BLEND);
        self.use_color_only(gl);
        // A texture cannot be sampled while attached to the framebuffer being drawn to.
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::TEXTURE_2D,
            None,
            0,
        );
        let program = if let (Some(tags), Some(palette)) = (target.styles, &self.palette) {
            let program = self.scoped_program.unwrap();
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT1,
                glow::TEXTURE_2D,
                None,
                0,
            );
            self.bind_screen_pass(gl, program, vao, target.depth, [0, 0], true);
            // Unit 1 can still be in use by Multishadow during the opaque pass.
            for (unit, name, texture) in
                [(2, "u_style_tags", tags), (3, "u_styles", palette.texture)]
            {
                gl.active_texture(glow::TEXTURE0 + unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.uniform_1_i32(gl.get_uniform_location(program, name).as_ref(), unit as i32);
            }
            gl.uniform_1_i32(
                gl.get_uniform_location(program, "u_style_columns").as_ref(),
                palette.columns,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(program, "u_scene_depth").as_ref(),
                scene_depth,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(program, "u_width").as_ref(),
                palette.max_width,
            );
            program
        } else {
            let program = self.program;
            let settings = settings.unwrap();
            self.bind_screen_pass(gl, program, vao, target.depth, [0, 0], true);
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(program, "u_color").as_ref(),
                settings.color.as_ref(),
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(program, "u_width").as_ref(),
                settings.width,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(program, "u_depth_jump").as_ref(),
                settings.depth_jump * scene_depth,
            );
            program
        };
        gl.uniform_2_f32(
            gl.get_uniform_location(program, "u_projection_depth")
                .as_ref(),
            projection.z_axis.z,
            projection.w_axis.z,
        );
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
        if let Some(tags) = target.styles {
            for unit in [2, 3] {
                gl.active_texture(glow::TEXTURE0 + unit);
                gl.bind_texture(glow::TEXTURE_2D, None);
            }
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT1,
                glow::TEXTURE_2D,
                Some(tags),
                0,
            );
            gl.active_texture(glow::TEXTURE0);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::TEXTURE_2D,
            Some(target.depth),
            0,
        );
        gl.enable(glow::DEPTH_TEST);
    }

    pub(super) unsafe fn finish(
        &self,
        gl: &glow::Context,
        vao: glow::VertexArray,
        destination: Destination,
    ) {
        let target = self.target.as_ref().unwrap();
        destination.restore(gl);
        let [x, y, _, _] = destination.viewport;
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::BLEND);
        // Drawing the color texture also works for a multisampled destination on WebGL2.
        self.bind_screen_pass(gl, self.program, vao, target.color, [x, y], false);
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }

    unsafe fn bind_screen_pass(
        &self,
        gl: &glow::Context,
        program: glow::Program,
        vao: glow::VertexArray,
        texture: glow::Texture,
        [x, y]: [i32; 2],
        outline: bool,
    ) {
        gl.use_program(Some(program));
        gl.bind_vertex_array(Some(vao));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.uniform_1_i32(gl.get_uniform_location(program, "u_texture").as_ref(), 0);
        gl.uniform_1_i32(
            gl.get_uniform_location(program, "u_outline").as_ref(),
            i32::from(outline),
        );
        gl.uniform_2_i32(gl.get_uniform_location(program, "u_origin").as_ref(), x, y);
    }

    unsafe fn release_target(&mut self, gl: &glow::Context) {
        if let Some(target) = self.target.take() {
            target.delete(gl);
        }
    }

    pub(super) unsafe fn release(&mut self, gl: &glow::Context) {
        self.release_target(gl);
        if let Some(palette) = self.palette.take() {
            gl.delete_texture(palette.texture);
        }
    }

    pub(super) unsafe fn destroy(mut self, gl: &glow::Context) {
        self.release(gl);
        gl.delete_program(self.program);
        if let Some(program) = self.scoped_program {
            gl.delete_program(program);
        }
    }
}
