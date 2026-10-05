use glam::Mat3;
use glam::Mat4;
use glam::Vec4;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

use eframe::{
    egui::{self, Vec2, mutex::Mutex},
    egui_glow, glow,
};
use glam::{EulerRot, Quat, Vec3};

use crate::Scene;
use crate::scene::{Animation, AutoRotate, DepthCue, Lighting};
use crate::shapes::Sphere;
use crate::shapes::SphereInstance;
use crate::shapes::Stick;
use crate::shapes::StickInstance;
use crate::utils::InstanceGroups;
use crate::utils::{Interpolatable, Logger};

const OPAQUE_ALPHA_THRESHOLD: f32 = 0.99;

#[derive(Clone, Copy)]
enum SceneRenderPass {
    Opaque = 0,
    TransparentDepth = 1,
    TransparentColor = 2,
}

impl SceneRenderPass {
    fn draws_transparent(self) -> bool {
        !matches!(self, Self::Opaque)
    }
}

#[derive(Clone, Copy, Default)]
struct AlphaCoverage {
    opaque: bool,
    transparent: bool,
}

impl AlphaCoverage {
    fn include(&mut self, alpha: f32) {
        if alpha < OPAQUE_ALPHA_THRESHOLD {
            self.transparent = true;
        } else {
            self.opaque = true;
        }
    }

    fn is_drawn_in(self, pass: SceneRenderPass) -> bool {
        if pass.draws_transparent() {
            self.transparent
        } else {
            self.opaque
        }
    }
}

#[derive(Clone, Copy, Default)]
struct SceneAlphaCoverage {
    meshes: AlphaCoverage,
    spheres: AlphaCoverage,
    sticks: AlphaCoverage,
}

impl SceneAlphaCoverage {
    fn has_transparent(self) -> bool {
        self.meshes.transparent || self.spheres.transparent || self.sticks.transparent
    }
}

struct SceneUniforms {
    model: Option<glow::UniformLocation>,
    view: Option<glow::UniformLocation>,
    projection: Option<glow::UniformLocation>,
    normal_matrix: Option<glow::UniformLocation>,
    light_pos: Option<glow::UniformLocation>,
    view_pos: Option<glow::UniformLocation>,
    light_color: Option<glow::UniformLocation>,
    light_intensity: Option<glow::UniformLocation>,
    ambient_light_color: Option<glow::UniformLocation>,
    ambient_light_intensity: Option<glow::UniformLocation>,
    diffuse_light_intensity: Option<glow::UniformLocation>,
    specular_light_intensity: Option<glow::UniformLocation>,
    render_pass: Option<glow::UniformLocation>,
    depth_cue_enabled: Option<glow::UniformLocation>,
    depth_cue_color: Option<glow::UniformLocation>,
    depth_cue_range: Option<glow::UniformLocation>,
}

impl SceneUniforms {
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn new(gl: &glow::Context, program: glow::Program) -> Self {
        use glow::HasContext as _;

        Self {
            model: gl.get_uniform_location(program, "u_model"),
            view: gl.get_uniform_location(program, "u_view"),
            projection: gl.get_uniform_location(program, "u_projection"),
            normal_matrix: gl.get_uniform_location(program, "u_normal_matrix"),
            light_pos: gl.get_uniform_location(program, "u_light_pos"),
            view_pos: gl.get_uniform_location(program, "u_view_pos"),
            light_color: gl.get_uniform_location(program, "u_light_color"),
            light_intensity: gl.get_uniform_location(program, "u_light_intensity"),
            ambient_light_color: gl.get_uniform_location(program, "u_ambient_light_color"),
            ambient_light_intensity: gl.get_uniform_location(program, "u_ambient_light_intensity"),
            diffuse_light_intensity: gl.get_uniform_location(program, "u_diffuse_light_intensity"),
            specular_light_intensity: gl
                .get_uniform_location(program, "u_specular_light_intensity"),
            render_pass: gl.get_uniform_location(program, "u_render_pass"),
            depth_cue_enabled: gl.get_uniform_location(program, "u_depth_cue_enabled"),
            depth_cue_color: gl.get_uniform_location(program, "u_depth_cue_color"),
            depth_cue_range: gl.get_uniform_location(program, "u_depth_cue_range"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SceneBounds {
    min: Vec3,
    max: Vec3,
}

impl SceneBounds {
    fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    fn include_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    fn include_sphere(&mut self, center: Vec3, radius: f32) {
        let radius = Vec3::splat(radius.max(0.0));
        self.include_point(center - radius);
        self.include_point(center + radius);
    }

    fn is_empty(self) -> bool {
        !self.min.is_finite() || !self.max.is_finite()
    }

    fn depth_range(self, model: Mat4, view: Mat4, depth_cue: DepthCue) -> [f32; 2] {
        if self.is_empty() {
            return [0.0, 1.0];
        }

        let mut near_depth = f32::INFINITY;
        let mut far_depth = f32::NEG_INFINITY;
        for x in [self.min.x, self.max.x] {
            for y in [self.min.y, self.max.y] {
                for z in [self.min.z, self.max.z] {
                    let eye = view * model * Vec3::new(x, y, z).extend(1.0);
                    let depth = -eye.z;
                    near_depth = near_depth.min(depth);
                    far_depth = far_depth.max(depth);
                }
            }
        }

        if !near_depth.is_finite() || !far_depth.is_finite() || far_depth <= near_depth {
            return [0.0, 1.0];
        }

        [
            near_depth + (far_depth - near_depth) * depth_cue.start,
            near_depth + (far_depth - near_depth) * depth_cue.end,
        ]
    }
}

/// Average repaint frequency over half-second windows using egui's monotonic time.
#[derive(Default)]
struct FrameRate {
    started_at: Option<f64>,
    last_tick: Option<f64>,
    last_activity: Option<f64>,
    frames: u32,
    fps: Option<f64>,
}

impl FrameRate {
    /// Returns the remaining idle deadline, never extending it for diagnostic paints.
    fn tick(&mut self, now: f64, activity: bool) -> Option<std::time::Duration> {
        if activity && !self.last_tick.is_some_and(|last| now <= last) {
            self.last_tick = Some(now);
            if self.last_activity.is_none_or(|last| now - last >= 1.0) {
                self.started_at = Some(now);
                self.frames = 0;
                self.fps = None;
            } else if let Some(start) = self.started_at {
                self.frames += 1;
                let elapsed = now - start;
                if elapsed >= 0.5 {
                    self.fps = Some(self.frames as f64 / elapsed);
                    self.frames = 0;
                    self.started_at = Some(now);
                }
            }
            self.last_activity = Some(now);
        }
        let last = self.last_activity?;
        let remaining = 1.0 - (now - last);
        if remaining <= 0.0 {
            self.fps = Some(0.0);
            self.last_activity = None;
            self.started_at = None;
            self.frames = 0;
            return None;
        }
        Some(std::time::Duration::from_secs_f64(remaining))
    }
}

pub struct Canvas<L: Logger> {
    shader: Arc<Mutex<Shader>>,
    camera_state: CameraState,
    animation: Option<Animation>,
    auto_rotate: AutoRotate,
    interpolate_enabled: bool,
    animation_start_time: Option<f64>,
    auto_rotate_last_time: Option<f64>,
    last_frame_id: Option<usize>,
    logger: L,
    camera_parameter_logging: bool,
    last_logged_camera_state: CameraState,
    last_camera_log_time: Option<f64>,
    show_fps: bool,
    show_camera_parameters: bool,
    frame_rate: FrameRate,
    diagnostic_activity: bool,
    last_overlay_rect: Option<egui::Rect>,
}

impl<L: Logger> Canvas<L> {
    pub fn new(gl: Arc<eframe::glow::Context>, scene: &Scene, logger: L) -> Option<Self> {
        let camera_state = scene.camera_state.unwrap_or(CameraState::default());
        Some(Self {
            shader: Arc::new(Mutex::new(Shader::new(&gl, scene)?)),
            camera_state,
            animation: None,
            auto_rotate: scene.auto_rotate,
            interpolate_enabled: false,
            animation_start_time: None,
            auto_rotate_last_time: None,
            last_frame_id: None,
            logger,
            camera_parameter_logging: false,
            last_logged_camera_state: camera_state,
            last_camera_log_time: None,
            show_fps: false,
            show_camera_parameters: false,
            frame_rate: FrameRate::default(),
            diagnostic_activity: true,
            last_overlay_rect: None,
        })
    }

    pub fn new_play(
        gl: Arc<eframe::glow::Context>,
        animation: Animation,
        logger: L,
    ) -> Option<Self> {
        if animation.frames.is_empty() {
            unreachable!("Animation must have at least one frame");
        }
        let init_frame = &animation.frames[0];
        let camera_state = init_frame.camera_state.unwrap_or(CameraState::default());
        Some(Self {
            shader: Arc::new(Mutex::new(Shader::new(&gl, init_frame)?)),
            camera_state,
            interpolate_enabled: animation.interpolate,
            auto_rotate: init_frame.auto_rotate,
            animation: Some(animation),
            animation_start_time: None,
            auto_rotate_last_time: None,
            last_frame_id: None,
            logger,
            camera_parameter_logging: false,
            last_logged_camera_state: camera_state,
            last_camera_log_time: None,
            show_fps: false,
            show_camera_parameters: false,
            frame_rate: FrameRate::default(),
            diagnostic_activity: true,
            last_overlay_rect: None,
        })
    }

    pub fn camera_parameter_logging(&mut self, enabled: bool) {
        self.diagnostic_activity = true;
        self.camera_parameter_logging = enabled;
        self.last_logged_camera_state = self.camera_state;
        self.last_camera_log_time = None;
    }

    pub fn show_fps(&mut self, enabled: bool) {
        self.diagnostic_activity = true;
        if self.show_fps != enabled {
            self.frame_rate = FrameRate::default();
        }
        self.show_fps = enabled;
    }

    pub fn show_camera_parameters(&mut self, enabled: bool) {
        self.diagnostic_activity = true;
        self.show_camera_parameters = enabled;
    }

    pub fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            egui::Vec2 {
                x: ui.available_width(),
                y: ui.available_height(),
            },
            egui::Sense::drag(),
        );

        let mut animation_playing = false;
        let static_scene = match self.animation.as_ref() {
            Some(animation) => animation.static_scene.as_ref(),
            None => None,
        };

        if let Some(animation) = self.animation.as_ref() {
            if animation.frames.is_empty() {
                self.logger.error("Animation contains no frames");
                return;
            }

            let now = ui.input(|i| i.time);
            if let None = self.animation_start_time {
                self.animation_start_time = Some(ui.input(|i| i.time));
            }

            let frame_count = animation.frames.len();
            let frame_duration = animation.interval as f64 / 1000.0; // 秒
            let total_duration = frame_duration * frame_count as f64;

            let elapsed = now - self.animation_start_time.unwrap();

            let mut is_finished = false;
            if animation.loops != -1 {
                let max_loops = animation.loops as usize;
                let max_time = total_duration * max_loops as f64;
                if elapsed >= max_time {
                    is_finished = true;
                }
            }
            if !is_finished {
                animation_playing = true;
                ui.ctx().request_repaint();
            }

            let anim_time = if animation.loops == -1 {
                elapsed % total_duration
            } else {
                elapsed % total_duration
            };

            let frame_index = (anim_time / frame_duration).floor() as usize;
            let frame_a_index = frame_index.min(frame_count - 1);
            let frame_b_index = if frame_a_index + 1 < frame_count {
                frame_a_index + 1
            } else {
                frame_a_index
            };

            let t = ((anim_time % frame_duration) / frame_duration) as f32;
            let frame_to_render: Option<Cow<Scene>> = if is_finished {
                Some(Cow::Borrowed(&animation.frames[frame_count - 1]))
            } else {
                if self.interpolate_enabled {
                    Some(Cow::Owned(animation.frames[frame_a_index].interpolate(
                        &animation.frames[frame_b_index],
                        t,
                        self.logger,
                    )))
                } else {
                    if Some(frame_a_index) != self.last_frame_id {
                        self.last_frame_id = Some(frame_a_index);
                        Some(Cow::Borrowed(&animation.frames[frame_a_index]))
                    } else {
                        self.last_frame_id = Some(frame_a_index);
                        None
                    }
                }
            };
            if let Some(frame) = frame_to_render {
                self.auto_rotate = frame.auto_rotate;
                self.shader.lock().update_scene(Some(&frame), static_scene);
            }
        }
        let zoom_disabled = self.shader.lock().zoom_disabled();
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);

        if !zoom_disabled && scroll_delta != 0.0 {
            let zoom_factor = 1.0 + scroll_delta * 0.001;

            self.camera_state.distance *= zoom_factor;

            self.camera_state.distance = self.camera_state.distance.clamp(0.1, 500.0);
        }

        if response.dragged() {
            self.camera_state.rotate(response.drag_motion());
        }

        if self.auto_rotate.enabled {
            ui.ctx().request_repaint();
            let now = ui.input(|i| i.time);
            if response.dragged() {
                self.auto_rotate_last_time = Some(now);
            } else if let Some(last_time) = self.auto_rotate_last_time {
                let dt = (now - last_time) as f32;
                if dt > 0.0 {
                    self.camera_state
                        .rotate_horizontally_by_degrees(self.auto_rotate.speed * dt);
                }
                self.auto_rotate_last_time = Some(now);
            } else {
                self.auto_rotate_last_time = Some(now);
            }
        } else {
            self.auto_rotate_last_time = None;
        }

        self.log_camera_parameters_if_changed(ui);

        // Clone locals so we can move them into the paint callback:
        let shader = self.shader.clone();

        let aspect_ratio = rect.width() / rect.height();
        let camera_state = self.camera_state.clone();

        let cb = egui_glow::CallbackFn::new(move |_info, painter| {
            shader
                .lock()
                .paint(painter.gl(), aspect_ratio, &camera_state, true);
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
        self.paint_overlay(ui, rect, animation_playing || self.auto_rotate.enabled);
    }

    fn paint_overlay(&mut self, ui: &egui::Ui, rect: egui::Rect, playing: bool) {
        if !self.show_fps && !self.show_camera_parameters {
            return;
        }
        let mut text = String::new();
        if self.show_fps {
            let activity = playing
                | std::mem::take(&mut self.diagnostic_activity)
                | (self.last_overlay_rect != Some(rect))
                | ui.input(|input| {
                    input
                        .events
                        .iter()
                        .any(|event| !matches!(event, egui::Event::Screenshot { .. }))
                });
            self.last_overlay_rect = Some(rect);
            // Screenshots and this idle wake only observe the readout; they do
            // not count as activity or keep renewing the idle deadline.
            if let Some(delay) = self.frame_rate.tick(ui.input(|input| input.time), activity) {
                ui.ctx().request_repaint_after(delay);
            }
            text = match self.frame_rate.fps {
                Some(fps) => format!("FPS: {fps:.1}"),
                None => "FPS: --".into(),
            };
        }
        if self.show_camera_parameters {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(
                &self
                    .camera_state
                    .format_orbit_parameters()
                    .replace(", distance=", "\ndistance=")
                    .replace(", target=", "\ntarget="),
            );
        }
        // Use only painter shapes: the overlay must not capture pointer input.
        let painter = ui.painter().with_clip_rect(rect);
        let origin = rect.min + egui::vec2(12.0, 12.0);
        let galley = painter.layout(
            text,
            egui::FontId::monospace(12.0),
            egui::Color32::WHITE,
            (rect.width() - 24.0).max(1.0),
        );
        painter.rect_filled(
            egui::Rect::from_min_size(origin, galley.size()).expand(4.0),
            4.0,
            egui::Color32::from_black_alpha(180),
        );
        painter.galley(origin, galley, egui::Color32::WHITE);
    }

    pub fn update_scene(&mut self, scene: &Scene) {
        self.diagnostic_activity = true;
        self.auto_rotate = scene.auto_rotate;
        self.shader.lock().update_scene(Some(scene), None);
    }

    pub fn transparent_background(&self) -> bool {
        self.shader.lock().transparent_background()
    }

    fn log_camera_parameters_if_changed(&mut self, ui: &egui::Ui) {
        if !self.camera_parameter_logging {
            return;
        }

        if !self
            .camera_state
            .is_visibly_different_from(&self.last_logged_camera_state)
        {
            return;
        }

        let now = ui.input(|i| i.time);
        if self
            .last_camera_log_time
            .is_some_and(|last| now - last < 0.2)
        {
            return;
        }

        self.logger.log(self.camera_state.format_orbit_parameters());
        self.last_logged_camera_state = self.camera_state;
        self.last_camera_log_time = Some(now);
    }
}

pub(super) struct Shader {
    program: Option<glow::Program>,
    program_bg: glow::Program,
    vao_bg: glow::VertexArray,
    program_sphere: Option<glow::Program>,
    program_stick: Option<glow::Program>,
    mesh_uniforms: Option<SceneUniforms>,
    sphere_uniforms: Option<SceneUniforms>,
    stick_uniforms: Option<SceneUniforms>,
    program_sphere_outline: Option<glow::Program>,
    program_stick_outline: Option<glow::Program>,
    vao_mesh: Option<glow::VertexArray>,
    vao_sphere: Option<glow::VertexArray>,
    vao_stick: Option<glow::VertexArray>,
    vao_sphere_outline: Option<glow::VertexArray>,
    vao_stick_outline: Option<glow::VertexArray>,
    camera_lighting: Lighting,
    vertex3d: Vec<Vertex3d>,
    indices: Vec<u32>,
    sphere_index_count: usize,
    stick_index_count: usize,
    background_color: Vec4,
    zoom_disabled: bool,
    depth_cue: DepthCue,
    depth_cue_color: Vec3,
    scene_bounds: SceneBounds,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    sphere_vbo: glow::Buffer,
    sphere_ebo: glow::Buffer,
    stick_vbo: glow::Buffer,
    stick_ebo: glow::Buffer,
    outline_quad_vbo: glow::Buffer,
    stick_outline_quad_vbo: glow::Buffer,
    sphere_instance_buffer: glow::Buffer,
    stick_instance_buffer: glow::Buffer,
    instance_groups: Option<InstanceGroups>,
    alpha_coverage: SceneAlphaCoverage,
    u_model: Mat4,
    u_normal_matrix: Mat3,
}

#[expect(unsafe_code)] // we need unsafe code to use glow
impl Shader {
    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn compile_program(
        gl: &glow::Context,
        shader_version: egui_glow::ShaderVersion,
        label: &str,
        vertex_shader: &str,
        fragment_shader: &str,
    ) -> glow::Program {
        use glow::HasContext as _;

        let program = gl.create_program().expect("Cannot create program");
        let shaders = [
            (glow::VERTEX_SHADER, vertex_shader),
            (glow::FRAGMENT_SHADER, fragment_shader),
        ]
        .iter()
        .map(|(shader_type, shader_source)| {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(
                shader,
                &format!(
                    "{}\n{}",
                    shader_version.version_declaration(),
                    shader_source
                ),
            );
            gl.compile_shader(shader);
            assert!(
                gl.get_shader_compile_status(shader),
                "Failed to compile {label} {shader_type}: {}",
                gl.get_shader_info_log(shader)
            );

            gl.attach_shader(program, shader);
            shader
        })
        .collect::<Vec<_>>();

        gl.link_program(program);
        assert!(
            gl.get_program_link_status(program),
            "{}",
            gl.get_program_info_log(program)
        );

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        program
    }

    pub(super) fn new(gl: &glow::Context, scene: &Scene) -> Option<Self> {
        use glow::HasContext as _;

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let background_color = scene_background_color(scene);
        let default_color = Vec4::new(1.0, 1.0, 1.0, 1.0);

        unsafe {
            if !shader_version.is_new_shader_interface() {
                println!(
                    "Custom 3D painting hasn't been ported to {:?}",
                    shader_version
                );
                return None;
            }

            let (vertex_shader_bg, fragment_shader_bg) = (
                include_str!("./bg_vertex.glsl"),
                include_str!("./bg_fragment.glsl"),
            );

            let program_bg = Self::compile_program(
                gl,
                shader_version,
                "custom_3d_glow_bg",
                vertex_shader_bg,
                fragment_shader_bg,
            );
            let vao_bg = gl
                .create_vertex_array()
                .expect("Cannot create background vertex array");

            // =========================
            // 4.1 Generate sphere mesh template
            // =========================
            // let template_sphere = Sphere::get_or_generate_sphere_mesh_template(2);
            let template_sphere = Sphere::get_or_generate_icosphere_mesh_template(3);

            let vertex3d_sphere: Vec<Vertex3d> = template_sphere
                .vertices
                .iter()
                .enumerate()
                .map(|(i, pos)| Vertex3d {
                    position: *pos,
                    normal: template_sphere.normals[i],
                    color: default_color.into(),
                    material: [0.65, 0.0],
                })
                .collect();

            let indices_sphere: Vec<u32> = template_sphere.indices.clone();

            // =========================
            // 4.2 Generate stick mesh template
            // =========================
            let template_stick = Stick::get_or_generate_cylinder_mesh_template(2);
            let vertex3d_stick: Vec<Vertex3d> = template_stick
                .vertices
                .iter()
                .enumerate()
                .map(|(i, pos)| Vertex3d {
                    position: *pos,
                    normal: template_stick.normals[i],
                    color: default_color.into(),
                    material: [0.65, 0.0],
                })
                .collect();

            let indices_stick: Vec<u32> = template_stick.indices.clone();

            // =========================
            // 5. Create buffers
            // =========================
            let vbo = gl.create_buffer().expect("Cannot create vertex buffer");
            let ebo = gl.create_buffer().expect("Cannot create element buffer");

            let sphere_vbo = gl
                .create_buffer()
                .expect("Cannot create sphere vertex buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(sphere_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertex3d_sphere),
                glow::STATIC_DRAW,
            );

            let sphere_ebo = gl
                .create_buffer()
                .expect("Cannot create sphere element buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(sphere_ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices_sphere),
                glow::STATIC_DRAW,
            );

            let stick_instance_buffer = gl
                .create_buffer()
                .expect("Cannot create stick instance buffer");

            let stick_vbo = gl
                .create_buffer()
                .expect("Cannot create stick vertex buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(stick_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertex3d_stick),
                glow::STATIC_DRAW,
            );

            let stick_ebo = gl
                .create_buffer()
                .expect("Cannot create stick element buffer");
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(stick_ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&indices_stick),
                glow::STATIC_DRAW,
            );

            let sphere_instance_buffer = gl
                .create_buffer()
                .expect("Cannot create sphere instance buffer");

            let outline_quad_vertices: [[f32; 2]; 6] = [
                [-1.0, -1.0],
                [1.0, -1.0],
                [-1.0, 1.0],
                [-1.0, 1.0],
                [1.0, -1.0],
                [1.0, 1.0],
            ];
            let stick_outline_quad_vertices: [[f32; 2]; 6] = [
                [-1.0, -1.0],
                [1.0, -1.0],
                [-1.0, 1.0],
                [-1.0, 1.0],
                [1.0, 1.0],
                [-1.0, -1.0],
            ];
            let outline_quad_vbo = gl
                .create_buffer()
                .expect("Cannot create outline quad vertex buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(outline_quad_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&outline_quad_vertices),
                glow::STATIC_DRAW,
            );
            let stick_outline_quad_vbo = gl
                .create_buffer()
                .expect("Cannot create stick outline quad vertex buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(stick_outline_quad_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&stick_outline_quad_vertices),
                glow::STATIC_DRAW,
            );

            // =========================
            // 8. Create shader instance struct
            // =========================
            let mut shader_instance = Self {
                program: None,
                program_bg,
                vao_bg,
                program_sphere: None,
                program_stick: None,
                mesh_uniforms: None,
                sphere_uniforms: None,
                stick_uniforms: None,
                program_sphere_outline: None,
                program_stick_outline: None,
                vertex3d: vec![],
                indices: vec![],
                camera_lighting: Lighting::default(),
                vao_mesh: None,
                vao_sphere: None,
                vao_stick: None,
                vao_sphere_outline: None,
                vao_stick_outline: None,
                sphere_instance_buffer,
                stick_instance_buffer,
                sphere_index_count: indices_sphere.len(),
                stick_index_count: indices_stick.len(),
                background_color,
                zoom_disabled: scene.zoom_disabled,
                depth_cue: scene.depth_cue,
                depth_cue_color: scene.depth_cue.color.unwrap_or(scene.background_color),
                scene_bounds: SceneBounds::empty(),
                vbo,
                ebo,
                sphere_vbo,
                sphere_ebo,
                stick_vbo,
                stick_ebo,
                outline_quad_vbo,
                stick_outline_quad_vbo,
                instance_groups: None,
                alpha_coverage: SceneAlphaCoverage::default(),
                u_model: scene.model_matrix(),
                u_normal_matrix: scene.normal_matrix(),
            };

            // =========================
            // 9. Update scene data
            // =========================
            shader_instance.update_scene(Some(scene), None);

            Some(shader_instance)
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn ensure_mesh_pipeline(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        if self.program.is_some() {
            return;
        }

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let program = Self::compile_program(
            gl,
            shader_version,
            "custom_3d_glow",
            include_str!("./vertex.glsl"),
            include_str!("./fragment.glsl"),
        );

        let vao = gl
            .create_vertex_array()
            .expect("Cannot create mesh vertex array");
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));

        let pos_loc = gl.get_attrib_location(program, "a_position").unwrap();
        let normal_loc = gl.get_attrib_location(program, "a_normal").unwrap();
        let color_loc = gl.get_attrib_location(program, "a_color").unwrap();
        let material_loc = gl.get_attrib_location(program, "a_material").unwrap();
        let stride_vertex_3d = std::mem::size_of::<Vertex3d>() as i32;

        gl.enable_vertex_attrib_array(pos_loc);
        gl.vertex_attrib_pointer_f32(pos_loc, 3, glow::FLOAT, false, stride_vertex_3d, 0);
        gl.enable_vertex_attrib_array(normal_loc);
        gl.vertex_attrib_pointer_f32(normal_loc, 3, glow::FLOAT, false, stride_vertex_3d, 3 * 4);
        gl.enable_vertex_attrib_array(color_loc);
        gl.vertex_attrib_pointer_f32(color_loc, 4, glow::FLOAT, false, stride_vertex_3d, 6 * 4);
        gl.enable_vertex_attrib_array(material_loc);
        gl.vertex_attrib_pointer_f32(
            material_loc,
            2,
            glow::FLOAT,
            false,
            stride_vertex_3d,
            10 * 4,
        );
        gl.bind_vertex_array(None);

        self.mesh_uniforms = Some(SceneUniforms::new(gl, program));
        self.program = Some(program);
        self.vao_mesh = Some(vao);
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn ensure_sphere_pipeline(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        if self.program_sphere.is_some() {
            return;
        }

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let program = Self::compile_program(
            gl,
            shader_version,
            "custom_3d_glow_sphere",
            include_str!("./vertex_sphere.glsl"),
            include_str!("./fragment.glsl"),
        );

        let vao = gl
            .create_vertex_array()
            .expect("Cannot create sphere vertex array");
        gl.bind_vertex_array(Some(vao));

        let pos_loc = gl.get_attrib_location(program, "a_position").unwrap();
        let normal_loc = gl.get_attrib_location(program, "a_normal").unwrap();
        let i_pos_loc = gl.get_attrib_location(program, "i_position").unwrap();
        let i_radius_loc = gl.get_attrib_location(program, "i_radius").unwrap();
        let i_color_loc = gl.get_attrib_location(program, "i_color").unwrap();
        let i_material_loc = gl.get_attrib_location(program, "i_material").unwrap();
        let stride_vertex_3d = std::mem::size_of::<Vertex3d>() as i32;

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.sphere_vbo));
        gl.enable_vertex_attrib_array(pos_loc);
        gl.vertex_attrib_pointer_f32(pos_loc, 3, glow::FLOAT, false, stride_vertex_3d, 0);
        gl.enable_vertex_attrib_array(normal_loc);
        gl.vertex_attrib_pointer_f32(normal_loc, 3, glow::FLOAT, false, stride_vertex_3d, 3 * 4);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.sphere_instance_buffer));
        let stride_instance = std::mem::size_of::<SphereInstance>() as i32;
        gl.enable_vertex_attrib_array(i_pos_loc);
        gl.vertex_attrib_pointer_f32(i_pos_loc, 3, glow::FLOAT, false, stride_instance, 0);
        gl.vertex_attrib_divisor(i_pos_loc, 1);
        gl.enable_vertex_attrib_array(i_radius_loc);
        gl.vertex_attrib_pointer_f32(i_radius_loc, 1, glow::FLOAT, false, stride_instance, 3 * 4);
        gl.vertex_attrib_divisor(i_radius_loc, 1);
        gl.enable_vertex_attrib_array(i_color_loc);
        gl.vertex_attrib_pointer_f32(i_color_loc, 4, glow::FLOAT, false, stride_instance, 4 * 4);
        gl.vertex_attrib_divisor(i_color_loc, 1);
        gl.enable_vertex_attrib_array(i_material_loc);
        gl.vertex_attrib_pointer_f32(
            i_material_loc,
            2,
            glow::FLOAT,
            false,
            stride_instance,
            8 * 4,
        );
        gl.vertex_attrib_divisor(i_material_loc, 1);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.sphere_ebo));
        gl.bind_vertex_array(None);

        self.sphere_uniforms = Some(SceneUniforms::new(gl, program));
        self.program_sphere = Some(program);
        self.vao_sphere = Some(vao);
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn ensure_stick_pipeline(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        if self.program_stick.is_some() {
            return;
        }

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let program = Self::compile_program(
            gl,
            shader_version,
            "custom_3d_glow_stick",
            include_str!("./vertex_stick.glsl"),
            include_str!("./fragment.glsl"),
        );

        let vao = gl
            .create_vertex_array()
            .expect("Cannot create stick vertex array");
        gl.bind_vertex_array(Some(vao));

        let pos_a_position = gl.get_attrib_location(program, "a_position").unwrap();
        let normal_a_position = gl.get_attrib_location(program, "a_normal").unwrap();
        let instance_i_start = gl.get_attrib_location(program, "i_start").unwrap();
        let instance_i_end = gl.get_attrib_location(program, "i_end").unwrap();
        let instance_i_radius = gl.get_attrib_location(program, "i_radius").unwrap();
        let instance_i_color = gl.get_attrib_location(program, "i_color").unwrap();
        let instance_i_material = gl.get_attrib_location(program, "i_material").unwrap();
        let stride_vertex_3d = std::mem::size_of::<Vertex3d>() as i32;

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_vbo));
        gl.enable_vertex_attrib_array(pos_a_position);
        gl.vertex_attrib_pointer_f32(pos_a_position, 3, glow::FLOAT, false, stride_vertex_3d, 0);
        gl.vertex_attrib_divisor(pos_a_position, 0);
        gl.enable_vertex_attrib_array(normal_a_position);
        gl.vertex_attrib_pointer_f32(
            normal_a_position,
            3,
            glow::FLOAT,
            false,
            stride_vertex_3d,
            3 * 4,
        );
        gl.vertex_attrib_divisor(normal_a_position, 0);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_instance_buffer));
        let stride_instance = std::mem::size_of::<StickInstance>() as i32;
        gl.enable_vertex_attrib_array(instance_i_start);
        gl.vertex_attrib_pointer_f32(instance_i_start, 3, glow::FLOAT, false, stride_instance, 0);
        gl.vertex_attrib_divisor(instance_i_start, 1);
        gl.enable_vertex_attrib_array(instance_i_end);
        gl.vertex_attrib_pointer_f32(
            instance_i_end,
            3,
            glow::FLOAT,
            false,
            stride_instance,
            3 * 4,
        );
        gl.vertex_attrib_divisor(instance_i_end, 1);
        gl.enable_vertex_attrib_array(instance_i_radius);
        gl.vertex_attrib_pointer_f32(
            instance_i_radius,
            1,
            glow::FLOAT,
            false,
            stride_instance,
            6 * 4,
        );
        gl.vertex_attrib_divisor(instance_i_radius, 1);
        gl.enable_vertex_attrib_array(instance_i_color);
        gl.vertex_attrib_pointer_f32(
            instance_i_color,
            4,
            glow::FLOAT,
            false,
            stride_instance,
            7 * 4,
        );
        gl.vertex_attrib_divisor(instance_i_color, 1);
        gl.enable_vertex_attrib_array(instance_i_material);
        gl.vertex_attrib_pointer_f32(
            instance_i_material,
            2,
            glow::FLOAT,
            false,
            stride_instance,
            11 * 4,
        );
        gl.vertex_attrib_divisor(instance_i_material, 1);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.stick_ebo));
        gl.bind_vertex_array(None);

        self.stick_uniforms = Some(SceneUniforms::new(gl, program));
        self.program_stick = Some(program);
        self.vao_stick = Some(vao);
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn ensure_sphere_outline_pipeline(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        if self.program_sphere_outline.is_some() {
            return;
        }

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let program = Self::compile_program(
            gl,
            shader_version,
            "custom_3d_glow_sphere_outline",
            include_str!("./vertex_sphere_outline.glsl"),
            include_str!("./fragment_sphere_outline.glsl"),
        );

        let vao = gl
            .create_vertex_array()
            .expect("Cannot create sphere outline vertex array");
        gl.bind_vertex_array(Some(vao));

        let corner_loc = gl.get_attrib_location(program, "a_corner").unwrap();
        let i_pos_loc = gl.get_attrib_location(program, "i_position").unwrap();
        let i_radius_loc = gl.get_attrib_location(program, "i_radius").unwrap();

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.outline_quad_vbo));
        gl.enable_vertex_attrib_array(corner_loc);
        gl.vertex_attrib_pointer_f32(corner_loc, 2, glow::FLOAT, false, 2 * 4, 0);
        gl.vertex_attrib_divisor(corner_loc, 0);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.sphere_instance_buffer));
        let stride_instance = std::mem::size_of::<SphereInstance>() as i32;
        gl.enable_vertex_attrib_array(i_pos_loc);
        gl.vertex_attrib_pointer_f32(i_pos_loc, 3, glow::FLOAT, false, stride_instance, 0);
        gl.vertex_attrib_divisor(i_pos_loc, 1);
        gl.enable_vertex_attrib_array(i_radius_loc);
        gl.vertex_attrib_pointer_f32(i_radius_loc, 1, glow::FLOAT, false, stride_instance, 3 * 4);
        gl.vertex_attrib_divisor(i_radius_loc, 1);

        gl.bind_vertex_array(None);

        self.program_sphere_outline = Some(program);
        self.vao_sphere_outline = Some(vao);
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn ensure_stick_outline_pipeline(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        if self.program_stick_outline.is_some() {
            return;
        }

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let program = Self::compile_program(
            gl,
            shader_version,
            "custom_3d_glow_stick_outline",
            include_str!("./vertex_stick_outline.glsl"),
            include_str!("./fragment_stick_outline.glsl"),
        );

        let vao = gl
            .create_vertex_array()
            .expect("Cannot create stick outline vertex array");
        gl.bind_vertex_array(Some(vao));

        let corner_loc = gl.get_attrib_location(program, "a_corner").unwrap();
        let instance_i_start = gl.get_attrib_location(program, "i_start").unwrap();
        let instance_i_end = gl.get_attrib_location(program, "i_end").unwrap();
        let instance_i_radius = gl.get_attrib_location(program, "i_radius").unwrap();

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_outline_quad_vbo));
        gl.enable_vertex_attrib_array(corner_loc);
        gl.vertex_attrib_pointer_f32(corner_loc, 2, glow::FLOAT, false, 2 * 4, 0);
        gl.vertex_attrib_divisor(corner_loc, 0);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_instance_buffer));
        let stride_instance = std::mem::size_of::<StickInstance>() as i32;
        gl.enable_vertex_attrib_array(instance_i_start);
        gl.vertex_attrib_pointer_f32(instance_i_start, 3, glow::FLOAT, false, stride_instance, 0);
        gl.vertex_attrib_divisor(instance_i_start, 1);
        gl.enable_vertex_attrib_array(instance_i_end);
        gl.vertex_attrib_pointer_f32(
            instance_i_end,
            3,
            glow::FLOAT,
            false,
            stride_instance,
            3 * 4,
        );
        gl.vertex_attrib_divisor(instance_i_end, 1);
        gl.enable_vertex_attrib_array(instance_i_radius);
        gl.vertex_attrib_pointer_f32(
            instance_i_radius,
            1,
            glow::FLOAT,
            false,
            stride_instance,
            6 * 4,
        );
        gl.vertex_attrib_divisor(instance_i_radius, 1);

        gl.bind_vertex_array(None);

        self.program_stick_outline = Some(program);
        self.vao_stick_outline = Some(vao);
    }

    fn update_scene(&mut self, scene_opt: Option<&Scene>, _static_scene_opt: Option<&Scene>) {
        let scene = if let Some(scene_data) = scene_opt {
            scene_data
        } else {
            return;
        };

        self.background_color = scene_background_color(scene);
        self.zoom_disabled = scene.zoom_disabled;
        self.depth_cue = scene.depth_cue;
        self.depth_cue_color = scene.depth_cue.color.unwrap_or(scene.background_color);
        self.vertex3d.clear();
        self.indices.clear();
        self.alpha_coverage = SceneAlphaCoverage::default();
        self.scene_bounds = SceneBounds::empty();

        let mut vertex_offset = 0u32;

        for mesh in scene._get_meshes() {
            self.vertex3d
                .extend(mesh.vertices.iter().enumerate().map(|(i, pos)| {
                    Vertex3d {
                        position: *pos,
                        normal: mesh.normals[i],
                        color: mesh
                            .colors
                            .as_ref()
                            .map(|x| x[i].into())
                            .unwrap_or([1.0; 4]),
                        material: mesh
                            .material_params
                            .as_ref()
                            .and_then(|params| params.get(i).copied())
                            .unwrap_or([0.65, 0.0]),
                    }
                }));

            self.indices
                .extend(mesh.indices.iter().map(|&i| i + vertex_offset));
            vertex_offset += mesh.vertices.len() as u32;
        }

        for vertex in &self.vertex3d {
            self.alpha_coverage.meshes.include(vertex.color[3]);
            self.scene_bounds.include_point(vertex.position);
        }

        let instance_groups = scene.get_instances_grouped();
        for sphere in &instance_groups.spheres {
            self.alpha_coverage.spheres.include(sphere.color[3]);
            self.scene_bounds
                .include_sphere(Vec3::from(sphere.position), sphere.radius);
        }
        for stick in &instance_groups.sticks {
            self.alpha_coverage.sticks.include(stick.color[3]);
            let radius = Vec3::splat(stick.radius.max(0.0));
            let start = Vec3::from(stick.start);
            let end = Vec3::from(stick.end);
            self.scene_bounds.include_point(start - radius);
            self.scene_bounds.include_point(start + radius);
            self.scene_bounds.include_point(end - radius);
            self.scene_bounds.include_point(end + radius);
        }
        self.instance_groups = Some(instance_groups);

        if let Some(lighting) = scene.camera_lights.as_ref() {
            self.camera_lighting = lighting.clone();
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn prepare_scene_geometry(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        if !self.indices.is_empty() {
            self.ensure_mesh_pipeline(gl);
            gl.bind_vertex_array(self.vao_mesh);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&self.vertex3d),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&self.indices),
                glow::DYNAMIC_DRAW,
            );
        }

        let has_spheres = self
            .instance_groups
            .as_ref()
            .is_some_and(|groups| !groups.spheres.is_empty());
        if has_spheres {
            self.ensure_sphere_pipeline(gl);
            let spheres = &self.instance_groups.as_ref().unwrap().spheres;
            gl.bind_vertex_array(self.vao_sphere);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.sphere_instance_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(spheres),
                glow::DYNAMIC_DRAW,
            );
        }

        let has_sticks = self
            .instance_groups
            .as_ref()
            .is_some_and(|groups| !groups.sticks.is_empty());
        if has_sticks {
            self.ensure_stick_pipeline(gl);
            let sticks = &self.instance_groups.as_ref().unwrap().sticks;
            gl.bind_vertex_array(self.vao_stick);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_instance_buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(sticks),
                glow::DYNAMIC_DRAW,
            );
        }

        gl.bind_vertex_array(None);
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn set_scene_uniforms(
        &self,
        gl: &glow::Context,
        program: glow::Program,
        uniforms: &SceneUniforms,
        u_view: Mat4,
        u_projection: Mat4,
        u_view_pos: Vec3,
        light_dir_world: Vec3,
        light_color: Vec3,
        light_intensity: f32,
        ambient_light_color: Vec3,
        ambient_light_intensity: f32,
        diffuse_light_intensity: f32,
        specular_light_intensity: f32,
        depth_cue_range: [f32; 2],
        pass: SceneRenderPass,
    ) {
        use glow::HasContext as _;

        gl.use_program(Some(program));
        gl.uniform_matrix_4_f32_slice(uniforms.model.as_ref(), false, self.u_model.as_ref());
        gl.uniform_matrix_4_f32_slice(uniforms.view.as_ref(), false, u_view.as_ref());
        gl.uniform_matrix_4_f32_slice(uniforms.projection.as_ref(), false, u_projection.as_ref());
        gl.uniform_matrix_3_f32_slice(
            uniforms.normal_matrix.as_ref(),
            false,
            self.u_normal_matrix.as_ref(),
        );
        gl.uniform_3_f32_slice(uniforms.light_pos.as_ref(), light_dir_world.as_ref());
        gl.uniform_3_f32_slice(uniforms.view_pos.as_ref(), u_view_pos.as_ref());
        gl.uniform_3_f32_slice(uniforms.light_color.as_ref(), light_color.as_ref());
        gl.uniform_1_f32(uniforms.light_intensity.as_ref(), light_intensity);
        gl.uniform_3_f32_slice(
            uniforms.ambient_light_color.as_ref(),
            ambient_light_color.as_ref(),
        );
        gl.uniform_1_f32(
            uniforms.ambient_light_intensity.as_ref(),
            ambient_light_intensity,
        );
        gl.uniform_1_f32(
            uniforms.diffuse_light_intensity.as_ref(),
            diffuse_light_intensity,
        );
        gl.uniform_1_f32(
            uniforms.specular_light_intensity.as_ref(),
            specular_light_intensity,
        );
        gl.uniform_1_i32(uniforms.render_pass.as_ref(), pass as i32);
        gl.uniform_1_i32(
            uniforms.depth_cue_enabled.as_ref(),
            i32::from(self.depth_cue.enabled),
        );
        gl.uniform_3_f32_slice(
            uniforms.depth_cue_color.as_ref(),
            self.depth_cue_color.as_ref(),
        );
        gl.uniform_2_f32_slice(uniforms.depth_cue_range.as_ref(), &depth_cue_range);
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    unsafe fn draw_scene_pass(
        &self,
        gl: &glow::Context,
        u_view: Mat4,
        u_projection: Mat4,
        u_view_pos: Vec3,
        light_dir_world: Vec3,
        light_color: Vec3,
        light_intensity: f32,
        ambient_light_color: Vec3,
        ambient_light_intensity: f32,
        diffuse_light_intensity: f32,
        specular_light_intensity: f32,
        depth_cue_range: [f32; 2],
        pass: SceneRenderPass,
    ) {
        use glow::HasContext as _;

        if !self.indices.is_empty() && self.alpha_coverage.meshes.is_drawn_in(pass) {
            let program = self.program.unwrap();
            let uniforms = self.mesh_uniforms.as_ref().unwrap();
            self.set_scene_uniforms(
                gl,
                program,
                uniforms,
                u_view,
                u_projection,
                u_view_pos,
                light_dir_world,
                light_color,
                light_intensity,
                ambient_light_color,
                ambient_light_intensity,
                diffuse_light_intensity,
                specular_light_intensity,
                depth_cue_range,
                pass,
            );
            gl.bind_vertex_array(self.vao_mesh);
            gl.draw_elements(
                glow::TRIANGLES,
                self.indices.len() as i32,
                glow::UNSIGNED_INT,
                0,
            );
        }

        if let Some(instance_groups) = self.instance_groups.as_ref() {
            if !instance_groups.spheres.is_empty() && self.alpha_coverage.spheres.is_drawn_in(pass)
            {
                let program = self.program_sphere.unwrap();
                let uniforms = self.sphere_uniforms.as_ref().unwrap();
                self.set_scene_uniforms(
                    gl,
                    program,
                    uniforms,
                    u_view,
                    u_projection,
                    u_view_pos,
                    light_dir_world,
                    light_color,
                    light_intensity,
                    ambient_light_color,
                    ambient_light_intensity,
                    diffuse_light_intensity,
                    specular_light_intensity,
                    depth_cue_range,
                    pass,
                );
                gl.bind_vertex_array(self.vao_sphere);
                gl.draw_elements_instanced(
                    glow::TRIANGLES,
                    self.sphere_index_count as i32,
                    glow::UNSIGNED_INT,
                    0,
                    instance_groups.spheres.len() as i32,
                );
            }

            if !instance_groups.sticks.is_empty() && self.alpha_coverage.sticks.is_drawn_in(pass) {
                let program = self.program_stick.unwrap();
                let uniforms = self.stick_uniforms.as_ref().unwrap();
                self.set_scene_uniforms(
                    gl,
                    program,
                    uniforms,
                    u_view,
                    u_projection,
                    u_view_pos,
                    light_dir_world,
                    light_color,
                    light_intensity,
                    ambient_light_color,
                    ambient_light_intensity,
                    diffuse_light_intensity,
                    specular_light_intensity,
                    depth_cue_range,
                    pass,
                );
                gl.bind_vertex_array(self.vao_stick);
                gl.draw_elements_instanced(
                    glow::TRIANGLES,
                    self.stick_index_count as i32,
                    glow::UNSIGNED_INT,
                    0,
                    instance_groups.sticks.len() as i32,
                );
            }
        }
    }

    pub(super) fn paint(
        &mut self,
        gl: &glow::Context,
        aspect_ratio: f32,
        camera_state: &CameraState,
        multisample: bool,
    ) {
        let (u_view, u_projection, u_view_pos) = camera_state.matrices(aspect_ratio);

        use glow::HasContext as _;

        let directional = self
            .camera_lighting
            .directionals
            .as_ref()
            .cloned()
            .unwrap_or_else(Lighting::default_directional);
        let light_dir_cam_space = directional.direction;
        let light_color_cam_space = directional.color;
        let light_intensity = directional.intensity;
        let diffuse_light_intensity = directional.diffuse;
        let specular_light_intensity = directional.specular;
        let ambient_light_color = self.camera_lighting.ambient.color;
        let ambient_light_intensity = self.camera_lighting.ambient.intensity;
        let rot = Mat3::from_mat4(u_view);
        let light_dir_world = rot.transpose() * light_dir_cam_space;
        let depth_cue_range = self
            .scene_bounds
            .depth_range(self.u_model, u_view, self.depth_cue);

        unsafe {
            gl.color_mask(true, true, true, true);
            gl.depth_mask(true);
            gl.depth_func(glow::LESS);
            gl.disable(glow::BLEND);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);

            gl.enable(glow::DEPTH_TEST);
            #[cfg(not(target_arch = "wasm32"))]
            if multisample {
                gl.enable(glow::MULTISAMPLE);
            } else {
                gl.disable(glow::MULTISAMPLE);
            }

            if self.transparent_background() {
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
            }
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            // === 绘制背景 ===
            gl.disable(glow::DEPTH_TEST); // ✅ 背景不需要深度
            gl.use_program(Some(self.program_bg));
            gl.bind_vertex_array(Some(self.vao_bg));
            gl.uniform_4_f32_slice(
                gl.get_uniform_location(self.program_bg, "background_color")
                    .as_ref(),
                self.background_color.as_ref(),
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            gl.bind_vertex_array(None);

            // === 绘制场景 ===
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.depth_func(glow::LESS);
            self.prepare_scene_geometry(gl);

            self.draw_scene_pass(
                gl,
                u_view,
                u_projection,
                u_view_pos,
                light_dir_world,
                light_color_cam_space,
                light_intensity,
                ambient_light_color,
                ambient_light_intensity,
                diffuse_light_intensity,
                specular_light_intensity,
                depth_cue_range,
                SceneRenderPass::Opaque,
            );

            if self.alpha_coverage.has_transparent() {
                // ChimeraX-style single-layer transparency: first record only
                // the nearest transparent depth, then shade that exact layer.
                gl.color_mask(false, false, false, false);
                gl.disable(glow::BLEND);
                gl.depth_func(glow::LESS);
                self.draw_scene_pass(
                    gl,
                    u_view,
                    u_projection,
                    u_view_pos,
                    light_dir_world,
                    light_color_cam_space,
                    light_intensity,
                    ambient_light_color,
                    ambient_light_intensity,
                    diffuse_light_intensity,
                    specular_light_intensity,
                    depth_cue_range,
                    SceneRenderPass::TransparentDepth,
                );

                gl.color_mask(true, true, true, true);
                gl.depth_func(glow::LEQUAL);
                gl.enable(glow::BLEND);
                gl.blend_func_separate(
                    glow::SRC_ALPHA,
                    glow::ONE_MINUS_SRC_ALPHA,
                    glow::ONE,
                    glow::ONE_MINUS_SRC_ALPHA,
                );
                self.draw_scene_pass(
                    gl,
                    u_view,
                    u_projection,
                    u_view_pos,
                    light_dir_world,
                    light_color_cam_space,
                    light_intensity,
                    ambient_light_color,
                    ambient_light_intensity,
                    diffuse_light_intensity,
                    specular_light_intensity,
                    depth_cue_range,
                    SceneRenderPass::TransparentColor,
                );

                gl.disable(glow::BLEND);
                gl.depth_func(glow::LESS);
            }

            if self
                .instance_groups
                .as_ref()
                .is_some_and(|groups| !groups.outlines.is_empty())
            {
                gl.disable(glow::CULL_FACE);
                gl.depth_mask(false);
                gl.depth_func(glow::LEQUAL);
                gl.enable(glow::BLEND);
                gl.blend_func_separate(
                    glow::SRC_ALPHA,
                    glow::ONE_MINUS_SRC_ALPHA,
                    glow::ONE,
                    glow::ONE_MINUS_SRC_ALPHA,
                );

                let outline_groups = self
                    .instance_groups
                    .as_ref()
                    .map(|groups| groups.outlines.clone())
                    .unwrap_or_default();

                for outline_group in outline_groups {
                    let outline_width = outline_group.settings.width;
                    if outline_width <= 0.0 {
                        continue;
                    }

                    if !outline_group.spheres.is_empty() {
                        self.ensure_sphere_outline_pipeline(gl);
                        let program = self.program_sphere_outline.unwrap();

                        gl.use_program(Some(program));
                        gl.uniform_matrix_4_f32_slice(
                            gl.get_uniform_location(program, "u_model").as_ref(),
                            false,
                            (self.u_model).as_ref(),
                        );
                        gl.uniform_matrix_4_f32_slice(
                            gl.get_uniform_location(program, "u_view").as_ref(),
                            false,
                            (u_view).as_ref(),
                        );
                        gl.uniform_matrix_4_f32_slice(
                            gl.get_uniform_location(program, "u_projection").as_ref(),
                            false,
                            (u_projection).as_ref(),
                        );
                        gl.uniform_1_f32(
                            gl.get_uniform_location(program, "u_outline_width").as_ref(),
                            outline_width,
                        );
                        gl.uniform_3_f32_slice(
                            gl.get_uniform_location(program, "u_outline_color").as_ref(),
                            outline_group.settings.color.as_ref(),
                        );

                        gl.bind_vertex_array(self.vao_sphere_outline);
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.sphere_instance_buffer));
                        gl.buffer_data_u8_slice(
                            glow::ARRAY_BUFFER,
                            bytemuck::cast_slice(&outline_group.spheres),
                            glow::DYNAMIC_DRAW,
                        );
                        gl.draw_arrays_instanced(
                            glow::TRIANGLES,
                            0,
                            6,
                            outline_group.spheres.len() as i32,
                        );
                    }

                    if !outline_group.sticks.is_empty() {
                        self.ensure_stick_outline_pipeline(gl);
                        let program = self.program_stick_outline.unwrap();

                        gl.use_program(Some(program));
                        gl.uniform_matrix_4_f32_slice(
                            gl.get_uniform_location(program, "u_model").as_ref(),
                            false,
                            (self.u_model).as_ref(),
                        );
                        gl.uniform_matrix_4_f32_slice(
                            gl.get_uniform_location(program, "u_view").as_ref(),
                            false,
                            (u_view).as_ref(),
                        );
                        gl.uniform_matrix_4_f32_slice(
                            gl.get_uniform_location(program, "u_projection").as_ref(),
                            false,
                            (u_projection).as_ref(),
                        );
                        gl.uniform_1_f32(
                            gl.get_uniform_location(program, "u_outline_width").as_ref(),
                            outline_width,
                        );
                        gl.uniform_3_f32_slice(
                            gl.get_uniform_location(program, "u_outline_color").as_ref(),
                            outline_group.settings.color.as_ref(),
                        );

                        gl.bind_vertex_array(self.vao_stick_outline);
                        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_instance_buffer));
                        gl.buffer_data_u8_slice(
                            glow::ARRAY_BUFFER,
                            bytemuck::cast_slice(&outline_group.sticks),
                            glow::DYNAMIC_DRAW,
                        );
                        gl.draw_arrays_instanced(
                            glow::TRIANGLES,
                            0,
                            6,
                            outline_group.sticks.len() as i32,
                        );
                    }
                }

                gl.depth_mask(true);
                gl.disable(glow::BLEND);
                gl.enable(glow::CULL_FACE);
                gl.cull_face(glow::BACK);
            }

            gl.color_mask(true, true, true, true);
            gl.depth_mask(true);
            gl.depth_func(glow::LESS);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }

    pub(super) fn set_background_color(&mut self, background_color: Vec4) {
        self.background_color = background_color;
    }

    pub(super) fn transparent_background(&self) -> bool {
        self.background_color.w <= 0.0
    }

    pub(super) fn zoom_disabled(&self) -> bool {
        self.zoom_disabled
    }
}

fn scene_background_color(scene: &Scene) -> Vec4 {
    if scene.transparent_background {
        Vec4::ZERO
    } else {
        scene.background_color.extend(1.0)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct CameraState {
    pub target: Vec3,
    pub distance: f32,
    pub rotation: Quat,
    pub fov: f32,
}

impl CameraState {
    pub fn new(distance: f32) -> Self {
        Self {
            target: Vec3::ZERO,
            distance,
            rotation: Quat::IDENTITY, // no rotation
            fov: 15.0,
        }
    }

    pub fn from_orbit_angles(
        azimuth: f32,
        elevation: f32,
        roll: f32,
        distance: f32,
        target: [f32; 3],
        fov: f32,
    ) -> Self {
        let rotation = Quat::from_rotation_y(azimuth.to_radians())
            * Quat::from_rotation_x(elevation.to_radians())
            * Quat::from_rotation_z(roll.to_radians());

        Self {
            target: Vec3::from(target),
            distance,
            rotation: rotation.normalize(),
            fov,
        }
    }

    pub fn matrices(&self, aspect: f32) -> (Mat4, Mat4, Vec3) {
        // Camera looks down -Z in local space
        let local_forward = Vec3::new(0.0, 0.0, -1.0);

        // Rotate the forward vector into world space
        let dir = self.rotation * local_forward;

        // Compute camera position
        let view_pos = self.target - dir * self.distance;

        // Up vector also comes from quaternion
        let up = self.rotation * Vec3::Y;

        let view = glam::camera::rh::view::look_at_mat4(view_pos, view_pos + dir, up);
        // Preserve the old perspective_rh convention: right-handed, Y-up, Z in [0, 1].
        let projection = glam::camera::rh::proj::directx::perspective(
            self.fov.to_radians(),
            aspect,
            0.1,
            2000.0,
        );

        (view, projection, view_pos)
    }

    pub fn rotate(&mut self, drag: Vec2) {
        // 灵敏度，可按需调整
        let sensitivity = 0.005;

        // 把屏幕拖动转换为两个角度
        let angle_x = -drag.x * sensitivity; // 水平：左右拖动
        let angle_y = -drag.y * sensitivity; // 垂直：上下拖动

        // 计算相机在世界空间的局部轴（world-space）
        // camera_right_world = rotation * X
        // camera_up_world    = rotation * Y
        let camera_right = self.rotation * Vec3::X;
        let camera_up = self.rotation * Vec3::Y;

        // 以相机的本地轴作为旋转轴，构造增量四元数（注意顺序）
        // 先绕相机的 up（左右拖动），再绕相机的 right（上下拖动）
        let q_yaw = Quat::from_axis_angle(camera_up, angle_x);
        let q_pitch = Quat::from_axis_angle(camera_right, angle_y);

        // 把“这次拖动产生的旋转” 先作用于现有旋转：）
        self.rotation = (q_yaw * q_pitch) * self.rotation;

        self.rotation = self.rotation.normalize();
    }

    pub fn rotate_by_degrees(&mut self, azimuth_delta: f32, elevation_delta: f32, roll_delta: f32) {
        let delta = Quat::from_rotation_y(azimuth_delta.to_radians())
            * Quat::from_rotation_x(elevation_delta.to_radians())
            * Quat::from_rotation_z(roll_delta.to_radians());

        self.rotation = (delta * self.rotation).normalize();
    }

    pub fn rotate_horizontally_by_degrees(&mut self, degrees: f32) {
        let camera_up = self.rotation * Vec3::Y;
        let delta = Quat::from_axis_angle(camera_up, degrees.to_radians());
        self.rotation = (delta * self.rotation).normalize();
    }

    pub fn orbit_angles_degrees(&self) -> (f32, f32, f32) {
        let (azimuth, elevation, roll) = self.rotation.to_euler(EulerRot::YXZ);
        (
            normalize_degrees(azimuth.to_degrees()),
            normalize_degrees(elevation.to_degrees()),
            normalize_degrees(roll.to_degrees()),
        )
    }

    pub fn format_orbit_parameters(&self) -> String {
        let (azimuth, elevation, roll) = self.orbit_angles_degrees();
        format!(
            "camera: azimuth={:.3}, elevation={:.3}, roll={:.3}, distance={:.3}, target=[{:.3}, {:.3}, {:.3}], fov={:.3}",
            azimuth,
            elevation,
            roll,
            self.distance,
            self.target.x,
            self.target.y,
            self.target.z,
            self.fov
        )
    }

    fn is_visibly_different_from(&self, other: &Self) -> bool {
        (self.target - other.target).length_squared() > 1.0e-8
            || (self.distance - other.distance).abs() > 1.0e-4
            || (self.fov - other.fov).abs() > 1.0e-4
            || self.rotation.dot(other.rotation).abs() < 0.999_999
    }
}

fn normalize_degrees(degrees: f32) -> f32 {
    let normalized = (degrees + 180.0).rem_euclid(360.0) - 180.0;
    if normalized == -180.0 {
        180.0
    } else {
        normalized
    }
}

impl Default for CameraState {
    fn default() -> Self {
        Self::new(35.0)
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug, Serialize, Deserialize)]
pub struct Vertex3d {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: [f32; 4],
    pub material: [f32; 2],
}

pub struct Light {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

#[cfg(test)]
mod tests {
    use super::{AlphaCoverage, CameraState, FrameRate, SceneBounds, SceneRenderPass};
    use crate::scene::{DepthCue, Lighting};
    use glam::{Mat4, Vec3};

    #[test]
    fn repaint_fps_averages_existing_frames_and_ignores_duplicate_passes() {
        let mut rate = FrameRate::default();
        rate.tick(10.0, true);
        assert!(rate.fps.is_none());
        for frame in 1..=30 {
            let now = 10.0 + frame as f64 / 60.0;
            rate.tick(now, true);
            rate.tick(now, true); // A second layout pass is not another rendered frame.
        }
        assert!((rate.fps.unwrap() - 60.0).abs() < 1e-6);
        assert_eq!(rate.tick(11.5, false), None);
        assert_eq!(rate.fps, Some(0.0));
    }

    #[test]
    fn idle_fps_refresh_does_not_count_frames_or_rearm_itself() {
        let mut rate = FrameRate::default();
        assert_eq!(
            rate.tick(0.0, true),
            Some(std::time::Duration::from_secs(1))
        );
        assert_eq!(
            rate.tick(0.5, false),
            Some(std::time::Duration::from_millis(500))
        );
        assert_eq!(rate.frames, 0);
        assert_eq!(rate.last_activity, Some(0.0));
        assert_eq!(rate.tick(1.0, false), None);
        assert_eq!(rate.fps, Some(0.0));
        assert_eq!(rate.tick(2.0, false), None);
        assert_eq!(rate.tick(3.0, false), None);
        assert_eq!(rate.frames, 0);
        assert_eq!(rate.fps, Some(0.0));
        // A scene update at the old deadline is genuine activity, not an idle frame.
        assert_eq!(
            rate.tick(3.0, true),
            Some(std::time::Duration::from_secs(1))
        );
        assert_eq!(rate.fps, None);
    }

    #[test]
    fn new_activity_defers_an_old_idle_timer_and_resumes_fps_statistics() {
        let mut rate = FrameRate::default();
        rate.tick(0.0, true);
        rate.tick(0.75, true);
        assert_eq!(
            rate.tick(1.0, false),
            Some(std::time::Duration::from_millis(750))
        );
        assert_ne!(rate.fps, Some(0.0));
        assert_eq!(rate.tick(1.75, false), None);
        assert_eq!(rate.fps, Some(0.0));
        rate.tick(2.0, true);
        for frame in 1..=30 {
            rate.tick(2.0 + frame as f64 / 60.0, true);
        }
        assert!((rate.fps.unwrap() - 60.0).abs() < 1e-6);
    }

    #[test]
    fn alpha_coverage_uses_chimerax_threshold() {
        let mut coverage = AlphaCoverage::default();
        coverage.include(0.5);
        coverage.include(0.989);

        assert!(!coverage.is_drawn_in(SceneRenderPass::Opaque));
        assert!(coverage.is_drawn_in(SceneRenderPass::TransparentDepth));
        assert!(coverage.is_drawn_in(SceneRenderPass::TransparentColor));

        coverage.include(0.99);
        coverage.include(1.0);

        assert!(coverage.is_drawn_in(SceneRenderPass::Opaque));
        assert!(coverage.is_drawn_in(SceneRenderPass::TransparentDepth));
    }

    #[test]
    fn scene_bounds_depth_range_uses_chimerax_fraction_model() {
        let mut bounds = SceneBounds::empty();
        bounds.include_point(Vec3::new(-1.0, -1.0, -10.0));
        bounds.include_point(Vec3::new(1.0, 1.0, -2.0));

        let range = bounds.depth_range(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            DepthCue {
                enabled: true,
                start: 0.5,
                end: 1.0,
                color: None,
            },
        );

        assert_eq!(range, [6.0, 10.0]);
    }

    #[test]
    fn camera_orbit_angles_round_trip_to_same_rotation() {
        let camera =
            CameraState::from_orbit_angles(135.0, -25.0, 12.0, 32.0, [1.0, 2.0, 3.0], 18.0);
        let (azimuth, elevation, roll) = camera.orbit_angles_degrees();
        let reconstructed = CameraState::from_orbit_angles(
            azimuth,
            elevation,
            roll,
            camera.distance,
            camera.target.to_array(),
            camera.fov,
        );

        assert!(camera.rotation.dot(reconstructed.rotation).abs() > 0.999_99);
    }

    #[test]
    fn camera_matrices_preserve_right_handed_view_space() {
        for camera in [
            CameraState::default(),
            CameraState::from_orbit_angles(135.0, -25.0, 12.0, 32.0, [1.0, 2.0, 3.0], 18.0),
        ] {
            let (view, _, eye) = camera.matrices(1.6);
            assert!(view.transform_point3(eye).abs_diff_eq(Vec3::ZERO, 1e-4));
            assert!(
                view.transform_point3(camera.target)
                    .abs_diff_eq(Vec3::new(0.0, 0.0, -camera.distance), 1e-4)
            );
            assert!(
                view.transform_vector3(camera.rotation * Vec3::Y)
                    .abs_diff_eq(Vec3::Y, 1e-4)
            );
        }
    }

    #[test]
    fn camera_projection_preserves_zero_to_one_depth_and_aspect() {
        let camera = CameraState::default();
        let aspect = 1.6;
        let (_, projection, _) = camera.matrices(aspect);
        let near = projection.project_point3(Vec3::new(0.0, 0.0, -0.1));
        let far = projection.project_point3(Vec3::new(0.0, 0.0, -2000.0));
        assert!(near.abs_diff_eq(Vec3::ZERO, 1e-5));
        assert!(far.abs_diff_eq(Vec3::Z, 1e-5));

        let half_height = (camera.fov.to_radians() * 0.5).tan();
        let top_right =
            projection.project_point3(Vec3::new(aspect * half_height, half_height, -1.0));
        assert!((top_right.x - 1.0).abs() < 1e-5);
        assert!((top_right.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn default_lighting_matches_previous_shader_constants() {
        let lighting = Lighting::default();
        let directional = lighting.directionals.as_ref().unwrap();

        assert_eq!(lighting.ambient.intensity, 0.55);
        assert_eq!(lighting.ambient.color.to_array(), [1.0, 0.97, 0.97]);
        assert_eq!(directional.diffuse, 0.65);
        assert_eq!(directional.specular, 1.0);
        assert_eq!(directional.intensity, 1.0);
        assert_eq!(directional.color.to_array(), [1.0, 0.97, 0.97]);
    }
}
