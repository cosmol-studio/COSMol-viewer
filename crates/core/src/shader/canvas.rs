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
use glam::{Quat, Vec3};

use crate::Scene;
use crate::scene::{Animation, Lighting};
use crate::shapes::Sphere;
use crate::shapes::SphereInstance;
use crate::shapes::Stick;
use crate::shapes::StickInstance;
use crate::utils::InstanceGroups;
use crate::utils::{Interpolatable, Logger};

pub struct Canvas<L: Logger> {
    shader: Arc<Mutex<Shader>>,
    camera_state: CameraState,
    animation: Option<Animation>,
    interpolate_enabled: bool,
    animation_start_time: Option<f64>,
    last_frame_id: Option<usize>,
    logger: L,
}

impl<L: Logger> Canvas<L> {
    pub fn new(gl: Arc<eframe::glow::Context>, scene: &Scene, logger: L) -> Option<Self> {
        let camera_state = scene.camera_state.clone();
        Some(Self {
            shader: Arc::new(Mutex::new(Shader::new(&gl, scene)?)),
            camera_state: camera_state.unwrap_or(CameraState::default()),
            animation: None,
            interpolate_enabled: false,
            animation_start_time: None,
            last_frame_id: None,
            logger,
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
        let camera_state = init_frame.camera_state;
        Some(Self {
            shader: Arc::new(Mutex::new(Shader::new(&gl, init_frame)?)),
            camera_state: camera_state.unwrap_or(CameraState::default()),
            interpolate_enabled: animation.interpolate,
            animation: Some(animation),
            animation_start_time: None,
            last_frame_id: None,
            logger,
        })
    }

    pub fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            egui::Vec2 {
                x: ui.available_width(),
                y: ui.available_height(),
            },
            egui::Sense::drag(),
        );

        let static_scene = match self.animation.as_ref() {
            Some(animation) => animation.static_scene.as_ref(),
            None => None,
        };

        if let Some(animation) = self.animation.as_ref() {
            ui.ctx().request_repaint();
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
                self.shader.lock().update_scene(Some(&frame), static_scene);
            }
        }
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);

        if scroll_delta != 0.0 {
            let zoom_factor = 1.0 + scroll_delta * 0.001;

            self.camera_state.distance *= zoom_factor;

            self.camera_state.distance = self.camera_state.distance.clamp(0.1, 500.0);
        }

        if response.dragged() {
            self.camera_state.rotate(response.drag_motion());
        }

        // Clone locals so we can move them into the paint callback:
        let shader = self.shader.clone();

        let aspect_ratio = rect.width() / rect.height();
        let camera_state = self.camera_state.clone();

        let cb = egui_glow::CallbackFn::new(move |_info, painter| {
            shader
                .lock()
                .paint(painter.gl(), aspect_ratio, &camera_state);
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }

    pub fn update_scene(&mut self, scene: &Scene) {
        self.shader.lock().update_scene(Some(scene), None);
    }
}

struct Shader {
    program: glow::Program,
    program_bg: glow::Program,
    program_sphere: glow::Program,
    program_stick: glow::Program,
    vao_mesh: glow::VertexArray,
    vao_sphere: glow::VertexArray,
    vao_stick: glow::VertexArray,
    camera_lighting: Lighting,
    vertex3d: Vec<Vertex3d>,
    indices: Vec<u32>,
    sphere_index_count: usize,
    stick_index_count: usize,
    background_color: Vec3,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    sphere_instance_buffer: glow::Buffer,
    stick_instance_buffer: glow::Buffer,
    instance_groups: Option<InstanceGroups>,
    u_model: Mat4,
    u_normal_matrix: Mat3,
}

#[expect(unsafe_code)] // we need unsafe code to use glow
impl Shader {
    fn new(gl: &glow::Context, scene: &Scene) -> Option<Self> {
        use glow::HasContext as _;

        let shader_version = egui_glow::ShaderVersion::get(gl);
        let background_color = scene.background_color;
        let default_color = Vec4::new(1.0, 1.0, 1.0, 1.0);

        unsafe {
            // =========================
            // 1. Create shader programs
            // =========================
            let program_bg = gl.create_program().expect("Cannot create program");
            let program = gl.create_program().expect("Cannot create program");
            let program_sphere = gl.create_program().expect("Cannot create program");
            let program_stick = gl.create_program().expect("Cannot create program");

            if !shader_version.is_new_shader_interface() {
                println!(
                    "Custom 3D painting hasn't been ported to {:?}",
                    shader_version
                );
                return None;
            }

            // =========================
            // 2. Load shader sources
            // =========================
            let (vertex_shader, fragment_shader) = (
                include_str!("./vertex.glsl"),
                include_str!("./fragment.glsl"),
            );

            let (vertex_shader_bg, fragment_shader_bg) = (
                include_str!("./bg_vertex.glsl"),
                include_str!("./bg_fragment.glsl"),
            );

            let vertex_sphere_shader = include_str!("./vertex_sphere.glsl");
            let vertex_stick_shader = include_str!("./vertex_stick.glsl");

            let shader = [
                (glow::VERTEX_SHADER, vertex_shader),
                (glow::FRAGMENT_SHADER, fragment_shader),
            ];

            let shader_bg = [
                (glow::VERTEX_SHADER, vertex_shader_bg),
                (glow::FRAGMENT_SHADER, fragment_shader_bg),
            ];

            let shader_sphere = [
                (glow::VERTEX_SHADER, vertex_sphere_shader),
                (glow::FRAGMENT_SHADER, fragment_shader),
            ];

            let shader_stick = [
                (glow::VERTEX_SHADER, vertex_stick_shader),
                (glow::FRAGMENT_SHADER, fragment_shader),
            ];

            println!("shader_version = {:?}", shader_version);

            // =========================
            // 3.1 Compile and link main shader
            // =========================
            let shaders: Vec<_> = shader
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
                        "Failed to compile custom_3d_glow {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );

                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

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

            // =========================
            // 3.2 Compile and link background shader
            // =========================
            let shaders_bg: Vec<_> = shader_bg
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader_bg");
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
                        "Failed to compile custom_3d_glow_bg {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );

                    gl.attach_shader(program_bg, shader);
                    shader
                })
                .collect();

            gl.link_program(program_bg);
            assert!(
                gl.get_program_link_status(program_bg),
                "{}",
                gl.get_program_info_log(program_bg)
            );

            for shader in shaders_bg {
                gl.detach_shader(program_bg, shader);
                gl.delete_shader(shader);
            }

            // =========================
            // 3.3 Compile and link sphere shader
            // =========================
            let shaders_sphere: Vec<_> = shader_sphere
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader_sphere");
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
                        "Failed to compile custom_3d_glow_sphere {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );

                    gl.attach_shader(program_sphere, shader);
                    shader
                })
                .collect();

            gl.link_program(program_sphere);
            assert!(
                gl.get_program_link_status(program_sphere),
                "{}",
                gl.get_program_info_log(program_sphere)
            );

            for shader in shaders_sphere {
                gl.detach_shader(program_sphere, shader);
                gl.delete_shader(shader);
            }

            // =========================
            // 3.4 Compile and link stick shader
            // =========================
            let shaders_stick: Vec<_> = shader_stick
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader_stick");
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
                        "Failed to compile custom_3d_glow_stick {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );

                    gl.attach_shader(program_stick, shader);
                    shader
                })
                .collect();

            gl.link_program(program_stick);
            assert!(
                gl.get_program_link_status(program_stick),
                "{}",
                gl.get_program_info_log(program_stick)
            );
            for shader in shaders_stick {
                gl.detach_shader(program_stick, shader);
                gl.delete_shader(shader);
            }

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

            // =========================
            // 6. Setup VAO for mesh
            // =========================
            let vao_mesh = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vao_mesh));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            let pos_loc = gl.get_attrib_location(program, "a_position").unwrap();
            let normal_loc = gl.get_attrib_location(program, "a_normal").unwrap();
            let color_loc = gl.get_attrib_location(program, "a_color").unwrap();
            let material_loc = gl.get_attrib_location(program, "a_material").unwrap();

            let stride_vertex_3d = std::mem::size_of::<Vertex3d>() as i32;

            gl.enable_vertex_attrib_array(pos_loc);
            gl.vertex_attrib_pointer_f32(pos_loc, 3, glow::FLOAT, false, stride_vertex_3d, 0);

            gl.enable_vertex_attrib_array(normal_loc);
            gl.vertex_attrib_pointer_f32(
                normal_loc,
                3,
                glow::FLOAT,
                false,
                stride_vertex_3d,
                3 * 4,
            );

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

            // =========================
            // 7.1 Setup VAO for instanced spheres
            // =========================

            let pos_loc = gl
                .get_attrib_location(program_sphere, "a_position")
                .unwrap();
            let normal_loc = gl.get_attrib_location(program_sphere, "a_normal").unwrap();
            let i_pos_loc = gl
                .get_attrib_location(program_sphere, "i_position")
                .unwrap();
            let i_radius_loc = gl.get_attrib_location(program_sphere, "i_radius").unwrap();
            let i_color_loc = gl.get_attrib_location(program_sphere, "i_color").unwrap();
            let i_material_loc = gl
                .get_attrib_location(program_sphere, "i_material")
                .unwrap();

            let vao_sphere = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vao_sphere));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(sphere_vbo));

            gl.enable_vertex_attrib_array(pos_loc); // a_position
            gl.vertex_attrib_pointer_f32(pos_loc, 3, glow::FLOAT, false, stride_vertex_3d, 0);

            gl.enable_vertex_attrib_array(normal_loc); // a_normal
            gl.vertex_attrib_pointer_f32(
                normal_loc,
                3,
                glow::FLOAT,
                false,
                stride_vertex_3d,
                3 * 4,
            );

            // per-instance attributes
            let sphere_instance_buffer = gl
                .create_buffer()
                .expect("Cannot create sphere instance buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(sphere_instance_buffer));

            let stride_instance = std::mem::size_of::<SphereInstance>() as i32;

            gl.enable_vertex_attrib_array(i_pos_loc); // i_position
            gl.vertex_attrib_pointer_f32(i_pos_loc, 3, glow::FLOAT, false, stride_instance, 0);
            gl.vertex_attrib_divisor(i_pos_loc, 1);

            gl.enable_vertex_attrib_array(i_radius_loc); // i_radius
            gl.vertex_attrib_pointer_f32(
                i_radius_loc,
                1,
                glow::FLOAT,
                false,
                stride_instance,
                3 * 4,
            );
            gl.vertex_attrib_divisor(i_radius_loc, 1);

            gl.enable_vertex_attrib_array(i_color_loc); // i_color
            gl.vertex_attrib_pointer_f32(
                i_color_loc,
                4,
                glow::FLOAT,
                false,
                stride_instance,
                4 * 4,
            );
            gl.vertex_attrib_divisor(i_color_loc, 1);

            gl.enable_vertex_attrib_array(i_material_loc); // i_material
            gl.vertex_attrib_pointer_f32(
                i_material_loc,
                2,
                glow::FLOAT,
                false,
                stride_instance,
                8 * 4,
            );
            gl.vertex_attrib_divisor(i_material_loc, 1);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(sphere_ebo));
            gl.bind_vertex_array(None);

            gl.use_program(Some(program));

            // =========================
            // 7.2 Setup VAO for instanced sticks
            // =========================
            let pos_a_position = gl.get_attrib_location(program_stick, "a_position").unwrap();
            let normal_a_position = gl.get_attrib_location(program_stick, "a_normal").unwrap();
            let instance_i_start = gl.get_attrib_location(program_stick, "i_start").unwrap();
            let instance_i_end = gl.get_attrib_location(program_stick, "i_end").unwrap();
            let instance_i_radius = gl.get_attrib_location(program_stick, "i_radius").unwrap();
            let instance_i_color = gl.get_attrib_location(program_stick, "i_color").unwrap();
            let instance_i_material = gl.get_attrib_location(program_stick, "i_material").unwrap();

            let vao_stick = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vao_stick));

            // per-vertex attributes
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(stick_vbo));
            gl.enable_vertex_attrib_array(pos_a_position); // a_position
            gl.vertex_attrib_pointer_f32(
                pos_a_position,
                3,
                glow::FLOAT,
                false,
                stride_vertex_3d,
                0,
            );
            gl.vertex_attrib_divisor(pos_a_position, 0);

            gl.enable_vertex_attrib_array(normal_a_position); // a_normal
            gl.vertex_attrib_pointer_f32(
                normal_a_position,
                3,
                glow::FLOAT,
                false,
                stride_vertex_3d,
                3 * 4,
            );
            gl.vertex_attrib_divisor(normal_a_position, 0);

            // per-instance attributes
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(stick_instance_buffer));
            let stride_instance = std::mem::size_of::<StickInstance>() as i32;

            gl.enable_vertex_attrib_array(instance_i_start); // i_start
            gl.vertex_attrib_pointer_f32(
                instance_i_start,
                3,
                glow::FLOAT,
                false,
                stride_instance,
                0,
            );
            gl.vertex_attrib_divisor(instance_i_start, 1);

            gl.enable_vertex_attrib_array(instance_i_end); // i_end
            gl.vertex_attrib_pointer_f32(
                instance_i_end,
                3,
                glow::FLOAT,
                false,
                stride_instance,
                3 * 4,
            );
            gl.vertex_attrib_divisor(instance_i_end, 1);

            gl.enable_vertex_attrib_array(instance_i_radius); // i_radius
            gl.vertex_attrib_pointer_f32(
                instance_i_radius,
                1,
                glow::FLOAT,
                false,
                stride_instance,
                6 * 4,
            );
            gl.vertex_attrib_divisor(instance_i_radius, 1);

            gl.enable_vertex_attrib_array(instance_i_color); // i_color
            gl.vertex_attrib_pointer_f32(
                instance_i_color,
                4,
                glow::FLOAT,
                false,
                stride_instance,
                7 * 4,
            );
            gl.vertex_attrib_divisor(instance_i_color, 1);

            gl.enable_vertex_attrib_array(instance_i_material); // i_material
            gl.vertex_attrib_pointer_f32(
                instance_i_material,
                2,
                glow::FLOAT,
                false,
                stride_instance,
                11 * 4,
            );
            gl.vertex_attrib_divisor(instance_i_material, 1);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(stick_ebo));
            gl.bind_vertex_array(None);

            gl.use_program(Some(program));

            // =========================
            // 8. Create shader instance struct
            // =========================
            let mut shader_instance = Self {
                program,
                program_bg,
                program_sphere,
                program_stick,
                vertex3d: vec![],
                indices: vec![],
                camera_lighting: Lighting::default(),
                vao_mesh,
                vao_sphere,
                vao_stick,
                sphere_instance_buffer,
                stick_instance_buffer,
                sphere_index_count: indices_sphere.len(),
                stick_index_count: indices_stick.len(),
                background_color,
                vbo,
                ebo,
                instance_groups: None,
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

    fn update_scene(&mut self, scene_opt: Option<&Scene>, static_scene_opt: Option<&Scene>) {
        let scene = if let Some(scene_data) = scene_opt {
            scene_data
        } else {
            return;
        };

        self.background_color = scene.background_color;
        self.vertex3d.clear();
        self.indices.clear();

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
                            .unwrap_or_default(),
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

        self.instance_groups = Some(scene.get_instances_grouped());

        if let Some(lighting) = scene.camera_lights.as_ref() {
            self.camera_lighting = lighting.clone();
        }
    }

    fn paint(&mut self, gl: &glow::Context, aspect_ratio: f32, camera_state: &CameraState) {
        let (u_view, u_projection, u_view_pos) = camera_state.matrices(aspect_ratio);

        use glow::HasContext as _;

        let light_dir_cam_space =
            if let Some(directionals) = self.camera_lighting.directionals.as_ref() {
                directionals.direction.clone()
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            };
        let light_color_cam_space =
            if let Some(directionals) = self.camera_lighting.directionals.as_ref() {
                directionals
                    .color
                    .clone()
                    .map(|x| x * directionals.intensity)
            } else {
                Vec3::new(0.0, 0.0, 0.0)
            };
        let rot = Mat3::from_mat4(u_view);
        let light_dir_world = rot.transpose() * light_dir_cam_space;

        unsafe {
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);

            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            #[cfg(not(target_arch = "wasm32"))]
            gl.enable(glow::MULTISAMPLE); // 开启多重采样

            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            // === 绘制背景 ===
            gl.disable(glow::DEPTH_TEST); // ✅ 背景不需要深度
            gl.use_program(Some(self.program_bg));
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program_bg, "background_color")
                    .as_ref(),
                self.background_color.as_ref(),
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 6);

            // === 绘制场景 ===
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true); // ✅ 关键：恢复写入深度缓冲区

            // gl.enable(glow::BLEND);
            // gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            gl.use_program(Some(self.program));

            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "u_model").as_ref(),
                false,
                (self.u_model).as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "u_view").as_ref(),
                false,
                (u_view).as_ref(),
            );
            gl.uniform_matrix_4_f32_slice(
                gl.get_uniform_location(self.program, "u_projection")
                    .as_ref(),
                false,
                (u_projection).as_ref(),
            );
            gl.uniform_matrix_3_f32_slice(
                gl.get_uniform_location(self.program, "u_normal_matrix")
                    .as_ref(),
                false,
                (self.u_normal_matrix).as_ref(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "u_light_pos")
                    .as_ref(),
                (light_dir_world).as_ref(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "u_view_pos").as_ref(),
                (u_view_pos).as_ref(),
            );

            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "u_light_color")
                    .as_ref(),
                light_color_cam_space.as_ref(),
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_light_intensity")
                    .as_ref(),
                1.0,
            );

            // 绑定并上传缓冲
            gl.bind_vertex_array(Some(self.vao_mesh));
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

            gl.draw_elements(
                glow::TRIANGLES,
                self.indices.len() as i32,
                glow::UNSIGNED_INT,
                0,
            );

            if let Some(instance_groups) = &self.instance_groups {
                gl.use_program(Some(self.program_sphere));
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_model")
                        .as_ref(),
                    false,
                    (self.u_model).as_ref(),
                );
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_view")
                        .as_ref(),
                    false,
                    (u_view).as_ref(),
                );
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_projection")
                        .as_ref(),
                    false,
                    (u_projection).as_ref(),
                );
                gl.uniform_matrix_3_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_normal_matrix")
                        .as_ref(),
                    false,
                    (self.u_normal_matrix).as_ref(),
                );

                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_light_pos")
                        .as_ref(),
                    (light_dir_world).as_ref(),
                );

                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_view_pos")
                        .as_ref(),
                    (u_view_pos).as_ref(),
                );

                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(self.program_sphere, "u_light_color")
                        .as_ref(),
                    light_color_cam_space.as_ref(),
                );

                gl.uniform_1_f32(
                    gl.get_uniform_location(self.program_sphere, "u_light_intensity")
                        .as_ref(),
                    1.0,
                );

                gl.bind_vertex_array(Some(self.vao_sphere));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.sphere_instance_buffer));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&instance_groups.spheres),
                    glow::DYNAMIC_DRAW,
                );

                gl.draw_elements_instanced(
                    glow::TRIANGLES,
                    self.sphere_index_count as i32,
                    glow::UNSIGNED_INT,
                    0,
                    instance_groups.spheres.len() as i32,
                );

                gl.use_program(Some(self.program_stick));
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_model")
                        .as_ref(),
                    false,
                    (self.u_model).as_ref(),
                );
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_view")
                        .as_ref(),
                    false,
                    (u_view).as_ref(),
                );
                gl.uniform_matrix_4_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_projection")
                        .as_ref(),
                    false,
                    (u_projection).as_ref(),
                );
                gl.uniform_matrix_3_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_normal_matrix")
                        .as_ref(),
                    false,
                    (self.u_normal_matrix).as_ref(),
                );
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_light_pos")
                        .as_ref(),
                    (light_dir_world).as_ref(),
                );
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_view_pos")
                        .as_ref(),
                    (u_view_pos).as_ref(),
                );
                gl.uniform_3_f32_slice(
                    gl.get_uniform_location(self.program_stick, "u_light_color")
                        .as_ref(),
                    light_color_cam_space.as_ref(),
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.program_stick, "u_light_intensity")
                        .as_ref(),
                    1.0,
                );
                gl.bind_vertex_array(Some(self.vao_stick));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.stick_instance_buffer));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(&instance_groups.sticks),
                    glow::DYNAMIC_DRAW,
                );

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

    pub fn matrices(&self, aspect: f32) -> (Mat4, Mat4, Vec3) {
        // Camera looks down -Z in local space
        let local_forward = Vec3::new(0.0, 0.0, -1.0);

        // Rotate the forward vector into world space
        let dir = self.rotation * local_forward;

        // Compute camera position
        let view_pos = self.target - dir * self.distance;

        // Up vector also comes from quaternion
        let up = self.rotation * Vec3::Y;

        let view = Mat4::look_at_rh(view_pos, view_pos + dir, up);
        let projection = Mat4::perspective_rh(self.fov.to_radians(), aspect, 0.1, 2000.0);

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
