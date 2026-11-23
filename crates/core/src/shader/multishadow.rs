use eframe::{egui_glow, glow, glow::HasContext as _};
use glam::{Mat4, Quat, Vec3, camera::rh};

use super::canvas::{GeometryDraw, GeometryKind, SceneUniforms, Shader};
use super::silhouette::Destination;
use crate::scene::Multishadow;
use crate::shapes::{Sphere, SphereInstance};

const TILE_SIZE: i32 = 128;
const TEXTURE_UNIT: u32 = 1;
const SAMPLES_BINDING: u32 = 0;

struct Atlas {
    texture: glow::Texture,
    framebuffer: glow::Framebuffer,
    samples: glow::Buffer,
    tiles: i32,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl Atlas {
    unsafe fn new(gl: &glow::Context, tiles: i32) -> Result<Self, String> {
        let texture = gl.create_texture()?;
        let framebuffer = gl
            .create_framebuffer()
            .inspect_err(|_| gl.delete_texture(texture))?;
        let samples = gl.create_buffer().inspect_err(|_| {
            gl.delete_framebuffer(framebuffer);
            gl.delete_texture(texture);
        })?;
        let atlas = Self {
            texture,
            framebuffer,
            samples,
            tiles,
        };
        let size = tiles * TILE_SIZE;
        gl.active_texture(glow::TEXTURE0 + TEXTURE_UNIT);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::DEPTH_COMPONENT24 as i32,
            size,
            size,
            0,
            glow::DEPTH_COMPONENT,
            glow::UNSIGNED_INT,
            glow::PixelUnpackData::Slice(None),
        );
        for (parameter, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_COMPARE_MODE, glow::COMPARE_REF_TO_TEXTURE),
            (glow::TEXTURE_COMPARE_FUNC, glow::LEQUAL),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value as i32);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE0);

        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        gl.draw_buffers(&[glow::NONE]);
        gl.read_buffer(glow::NONE);
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        if status != glow::FRAMEBUFFER_COMPLETE {
            atlas.delete(gl);
            return Err(format!(
                "Multishadow framebuffer is incomplete: 0x{status:x}"
            ));
        }
        Ok(atlas)
    }

    unsafe fn delete(self, gl: &glow::Context) {
        gl.delete_buffer(self.samples);
        gl.delete_framebuffer(self.framebuffer);
        gl.delete_texture(self.texture);
    }
}

// Array elements of GLSL's std140 ShadowSample are a mat4 and a vec4.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct ShadowSample {
    object_to_atlas: [f32; 16],
    direction: [f32; 4],
}

struct Programs {
    depth: glow::Program,
    depth_transform: Option<glow::UniformLocation>,
    sphere_template: Option<ShadowSphereTemplate>,
    lit: glow::Program,
    lit_uniforms: SceneUniforms,
    sampler: Option<glow::UniformLocation>,
    count: Option<glow::UniformLocation>,
    normal_bias: Option<glow::UniformLocation>,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl Programs {
    unsafe fn new(gl: &glow::Context, draw: &GeometryDraw) -> Self {
        let kind = draw.kind;
        let shader_version = egui_glow::ShaderVersion::get(gl);
        let source = kind.vertex_source();
        let depth = Shader::compile_program(
            gl,
            shader_version,
            "multishadow depth",
            source,
            include_str!("multishadow_depth.glsl"),
        );
        gl.use_program(Some(depth));
        let identity = Mat4::IDENTITY.to_cols_array();
        for name in ["u_model", "u_view"] {
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(depth, name).as_ref(),
                false,
                &identity,
            );
        }
        let defines = format!(
            "#define USE_MULTISHADOW\n#define MAX_SHADOW_DIRECTIONS {}\n",
            Multishadow::MAX_DIRECTIONS,
        );
        let lit = Shader::compile_program(
            gl,
            shader_version,
            "multishadow lighting",
            &format!("{defines}{source}"),
            &format!("{defines}{}", include_str!("fragment.glsl")),
        );
        let block = gl
            .get_uniform_block_index(lit, "MultishadowSamples")
            .unwrap();
        gl.uniform_block_binding(lit, block, SAMPLES_BINDING);
        let sphere_template = match kind {
            GeometryKind::Sphere => Some(ShadowSphereTemplate::new(
                gl,
                depth,
                draw.instances.unwrap().1,
            )),
            _ => None,
        };

        Self {
            depth,
            depth_transform: gl.get_uniform_location(depth, "u_projection"),
            sphere_template,
            lit,
            lit_uniforms: SceneUniforms::new(gl, lit),
            sampler: gl.get_uniform_location(lit, "u_shadow_atlas"),
            count: gl.get_uniform_location(lit, "u_shadow_count"),
            normal_bias: gl.get_uniform_location(lit, "u_shadow_normal_bias"),
        }
    }
}

struct ShadowSphereTemplate {
    vao: glow::VertexArray,
    vertices: glow::Buffer,
    indices: glow::Buffer,
    index_count: i32,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl ShadowSphereTemplate {
    unsafe fn new(gl: &glow::Context, program: glow::Program, instances: glow::Buffer) -> Self {
        // An 80-triangle sphere is sufficient for the 128px depth tiles. Keep the
        // same positions, radii and opacities as the full-resolution spheres.
        let template = Sphere::get_or_generate_icosphere_mesh_template(1);
        let vao = gl
            .create_vertex_array()
            .expect("Cannot create shadow sphere VAO");
        let vertices = gl
            .create_buffer()
            .expect("Cannot create shadow sphere vertices");
        let indices = gl
            .create_buffer()
            .expect("Cannot create shadow sphere indices");
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&template.vertices),
            glow::STATIC_DRAW,
        );
        let position = gl.get_attrib_location(program, "a_position").unwrap();
        gl.enable_vertex_attrib_array(position);
        gl.vertex_attrib_pointer_f32(position, 3, glow::FLOAT, false, size_of::<Vec3>() as i32, 0);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances));
        for (name, components, offset) in [
            (
                "i_position",
                3,
                std::mem::offset_of!(SphereInstance, position),
            ),
            ("i_radius", 1, std::mem::offset_of!(SphereInstance, radius)),
            ("i_color", 4, std::mem::offset_of!(SphereInstance, color)),
        ] {
            let location = gl.get_attrib_location(program, name).unwrap();
            gl.enable_vertex_attrib_array(location);
            gl.vertex_attrib_pointer_f32(
                location,
                components,
                glow::FLOAT,
                false,
                size_of::<SphereInstance>() as i32,
                offset as i32,
            );
            gl.vertex_attrib_divisor(location, 1);
        }
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            bytemuck::cast_slice(&template.indices),
            glow::STATIC_DRAW,
        );
        gl.bind_vertex_array(None);
        Self {
            vao,
            vertices,
            indices,
            index_count: template.indices.len() as i32,
        }
    }

    unsafe fn draw(&self, gl: &glow::Context, instances: i32) {
        gl.bind_vertex_array(Some(self.vao));
        gl.draw_elements_instanced(
            glow::TRIANGLES,
            self.index_count,
            glow::UNSIGNED_INT,
            0,
            instances,
        );
    }

    unsafe fn delete(self, gl: &glow::Context) {
        gl.delete_vertex_array(self.vao);
        gl.delete_buffer(self.vertices);
        gl.delete_buffer(self.indices);
    }
}

#[derive(Default)]
pub(super) struct MultishadowPass {
    atlas: Option<Atlas>,
    programs: [Option<Programs>; 3],
    count: u8,
    normal_bias: f32,
}

#[allow(unsafe_op_in_unsafe_fn)]
impl MultishadowPass {
    pub(super) unsafe fn rebuild(
        &mut self,
        gl: &glow::Context,
        destination: &Destination,
        settings: Multishadow,
        center: Vec3,
        radius: f32,
        draws: &[Option<GeometryDraw>; 3],
    ) -> Result<(), String> {
        let count = usize::from(settings.directions);
        let tiles = ((count - 1).isqrt() + 1) as i32;
        if !draws.iter().flatten().any(|draw| draw.opaque()) {
            self.release_atlas(gl);
            return Ok(());
        }
        if self.atlas.as_ref().is_none_or(|atlas| atlas.tiles != tiles) {
            self.release_atlas(gl);
            self.atlas = match Atlas::new(gl, tiles) {
                Ok(atlas) => Some(atlas),
                Err(error) => {
                    destination.restore(gl);
                    return Err(error);
                }
            };
        }
        for draw in draws.iter().flatten() {
            self.programs[draw.kind as usize].get_or_insert_with(|| Programs::new(gl, draw));
        }
        let atlas = self.atlas.as_ref().unwrap();
        let radius = radius * 1.05;
        let ortho =
            rh::proj::opengl::orthographic(-radius, radius, -radius, radius, radius, 3.0 * radius);
        let mut samples = [ShadowSample {
            object_to_atlas: [0.0; 16],
            direction: [0.0; 4],
        }; Multishadow::MAX_DIRECTIONS];

        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(atlas.framebuffer));
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_mask(true);
        gl.depth_func(glow::LESS);
        gl.enable(glow::POLYGON_OFFSET_FILL);
        gl.polygon_offset(1.0, 2.0);
        gl.clear_buffer_f32_slice(glow::DEPTH, 0, &[1.0]);
        for (index, sample) in samples.iter_mut().take(count).enumerate() {
            let direction = sample_direction(index, count);
            let up = if direction.y.abs() < 0.99 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let clip =
                ortho * rh::view::look_at_mat4(center + direction * (2.0 * radius), center, up);
            let x = index as i32 % tiles;
            let y = index as i32 / tiles;
            gl.viewport(x * TILE_SIZE, y * TILE_SIZE, TILE_SIZE, TILE_SIZE);
            for draw in draws.iter().flatten().filter(|draw| draw.opaque()) {
                let program = self.programs[draw.kind as usize].as_ref().unwrap();
                gl.use_program(Some(program.depth));
                gl.uniform_matrix_4_f32_slice(
                    program.depth_transform.as_ref(),
                    false,
                    clip.as_ref(),
                );
                if let Some(template) = &program.sphere_template {
                    template.draw(gl, draw.instances.unwrap().0);
                } else {
                    draw.draw(gl);
                }
            }
            let tile_scale = 1.0 / tiles as f32;
            let to_atlas = Mat4::from_scale_rotation_translation(
                Vec3::new(0.5 * tile_scale, 0.5 * tile_scale, 0.5),
                Quat::IDENTITY,
                Vec3::new(
                    (x as f32 + 0.5) * tile_scale,
                    (y as f32 + 0.5) * tile_scale,
                    0.5,
                ),
            );
            sample.object_to_atlas = (to_atlas * clip).to_cols_array();
            sample.direction = direction.extend(0.0).to_array();
        }
        gl.disable(glow::POLYGON_OFFSET_FILL);
        destination.restore(gl);
        gl.bind_buffer(glow::UNIFORM_BUFFER, Some(atlas.samples));
        gl.buffer_data_u8_slice(
            glow::UNIFORM_BUFFER,
            bytemuck::cast_slice(&samples),
            glow::DYNAMIC_DRAW,
        );
        gl.bind_buffer(glow::UNIFORM_BUFFER, None);
        self.count = settings.directions;
        self.normal_bias = radius / TILE_SIZE as f32;
        Ok(())
    }

    pub(super) unsafe fn bind(&self, gl: &glow::Context) {
        if let Some(atlas) = &self.atlas {
            gl.active_texture(glow::TEXTURE0 + TEXTURE_UNIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(atlas.texture));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_buffer_base(glow::UNIFORM_BUFFER, SAMPLES_BINDING, Some(atlas.samples));
        }
    }

    pub(super) unsafe fn unbind(&self, gl: &glow::Context) {
        if self.atlas.is_some() {
            gl.active_texture(glow::TEXTURE0 + TEXTURE_UNIT);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_buffer_base(glow::UNIFORM_BUFFER, SAMPLES_BINDING, None);
        }
    }

    pub(super) fn lighting_program(
        &self,
        kind: GeometryKind,
    ) -> Option<(glow::Program, &SceneUniforms)> {
        self.atlas.as_ref()?;
        let program = self.programs[kind as usize].as_ref()?;
        Some((program.lit, &program.lit_uniforms))
    }

    pub(super) unsafe fn set_uniforms(&self, gl: &glow::Context, kind: GeometryKind) {
        let program = self.programs[kind as usize].as_ref().unwrap();
        gl.uniform_1_i32(program.sampler.as_ref(), TEXTURE_UNIT as i32);
        gl.uniform_1_i32(program.count.as_ref(), i32::from(self.count));
        gl.uniform_1_f32(program.normal_bias.as_ref(), self.normal_bias);
    }

    pub(super) unsafe fn release_atlas(&mut self, gl: &glow::Context) {
        if let Some(atlas) = self.atlas.take() {
            atlas.delete(gl);
        }
    }

    pub(super) unsafe fn destroy(mut self, gl: &glow::Context) {
        self.release_atlas(gl);
        for program in self.programs.into_iter().flatten() {
            if let Some(template) = program.sphere_template {
                template.delete(gl);
            }
            gl.delete_program(program.depth);
            gl.delete_program(program.lit);
        }
    }
}

fn sample_direction(index: usize, count: usize) -> Vec3 {
    // Fibonacci sphere: equal-area latitude bands with a golden-angle azimuth step.
    const GOLDEN_ANGLE: f32 = 2.3999631;
    let y = 1.0 - 2.0 * (index as f32 + 0.5) / count as f32;
    let radius = (1.0 - y * y).sqrt();
    let (sin, cos) = (index as f32 * GOLDEN_ANGLE).sin_cos();
    Vec3::new(cos * radius, y, sin * radius)
}
