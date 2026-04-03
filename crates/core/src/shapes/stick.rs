use crate::Arc;
use dashmap::DashMap;
use eframe::glow;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use glam::{Vec3, Vec4};

use crate::{
    Shape,
    scene::Scene,
    shapes::sphere::MeshTemplate,
    utils::{Interaction, Interpolatable, Logger, Material, MeshData, Stylable},
};

static STICK_TEMPLATE_CACHE: Lazy<DashMap<(u32, StickCap), Arc<MeshTemplate>>> =
    Lazy::new(|| DashMap::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum StickCap {
    None,
    Flat,
    Round,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Stick {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub thickness_radius: f32,
    pub quality: u32,
    pub stick_cap: StickCap,

    pub style: Material,
    interaction: Interaction,
}

impl Interpolatable for Stick {
    fn interpolate(&self, other: &Self, t: f32, _logger: impl Logger) -> Self {
        Self {
            start: [
                self.start[0] * (1.0 - t) + other.start[0] * t,
                self.start[1] * (1.0 - t) + other.start[1] * t,
                self.start[2] * (1.0 - t) + other.start[2] * t,
            ],
            end: [
                self.end[0] * (1.0 - t) + other.end[0] * t,
                self.end[1] * (1.0 - t) + other.end[1] * t,
                self.end[2] * (1.0 - t) + other.end[2] * t,
            ],
            thickness_radius: self.thickness_radius * (1.0 - t) + other.thickness_radius * t,
            quality: ((self.quality as f32) * (1.0 - t) + (other.quality as f32) * t) as u32,
            style: self.style.clone(), // 直接 clone，或者实现 style 插值
            interaction: self.interaction.clone(),
            stick_cap: StickCap::None,
        }
    }
}

impl Into<Shape> for Stick {
    fn into(self) -> Shape {
        Shape::Stick(self)
    }
}

impl Stick {
    pub fn new(start: [f32; 3], end: [f32; 3], radius: f32) -> Self {
        Self {
            start,
            end,
            thickness_radius: radius,
            quality: 6,
            style: Material {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
            interaction: Default::default(),
            stick_cap: StickCap::None,
        }
    }

    pub fn set_thickness(mut self, thickness: f32) -> Self {
        self.thickness_radius = thickness;
        self
    }

    pub fn set_start(mut self, start: [f32; 3]) -> Self {
        self.start = start;
        self
    }

    pub fn set_end(mut self, end: [f32; 3]) -> Self {
        self.end = end;
        self
    }

    // fn clickable(mut self, val: bool) -> Self {
    //     self.interaction.clickable = val;
    //     self
    // }

    pub fn to_mesh(&self, scale: f32) -> MeshData {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();

        let segments = 10 * self.quality;
        let r = self.thickness_radius;

        let start = Vec3::from_array(self.start);
        let end = Vec3::from_array(self.end);
        let axis = end - start;
        let height = axis.length();

        let base_color = self.style.color.unwrap_or([1.0, 1.0, 1.0].into());
        let alpha = self.style.opacity.clamp(0.0, 1.0);
        let color = Vec4::new(base_color[0], base_color[1], base_color[2], alpha);

        build_cylinder_body(
            segments,
            r,
            height,
            color,
            &mut vertices,
            &mut normals,
            &mut indices,
            &mut colors,
        );

        let cap = self.stick_cap;

        match cap {
            StickCap::None => {}
            StickCap::Flat => {
                build_flat_cap(
                    0.0,
                    Vec3::NEG_Z,
                    segments,
                    r,
                    color,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    &mut colors,
                );
                build_flat_cap(
                    height,
                    Vec3::Z,
                    segments,
                    r,
                    color,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    &mut colors,
                );
            }
            StickCap::Round => {
                build_round_cap(
                    0.0,
                    -1.0,
                    segments,
                    self.quality,
                    r,
                    color,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    &mut colors,
                );
                build_round_cap(
                    height,
                    1.0,
                    segments,
                    self.quality,
                    r,
                    color,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    &mut colors,
                );
            }
        }

        let rotation = glam::Quat::from_rotation_arc(Vec3::Z, axis.normalize());

        for v in &mut vertices {
            *v = (rotation * *v + start) * scale;
        }

        for n in &mut normals {
            *n = rotation * *n;
        }

        MeshData {
            vertices,
            normals,
            indices,
            colors: Some(colors),
            transform: None,
            is_wireframe: self.style.wireframe,
        }
    }

    pub fn get_or_generate_cylinder_mesh_template(quality: u32) -> Arc<MeshTemplate> {
        let key = (quality, StickCap::None);

        if let Some(entry) = STICK_TEMPLATE_CACHE.get(&key) {
            return Arc::clone(entry.value());
        }

        let stick = Stick::new([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 1.0).set_thickness(1.0);

        let mesh = stick.to_mesh(1.0);

        let template = Arc::new(MeshTemplate {
            vertices: mesh.vertices,
            normals: mesh.normals,
            indices: mesh.indices,
        });

        STICK_TEMPLATE_CACHE.insert(key, Arc::clone(&template));
        template
    }

    pub fn to_instance(&self, scale: f32) -> StickInstance {
        let base_color = self.style.color.unwrap_or([1.0, 1.0, 1.0].into());
        let alpha = self.style.opacity.clamp(0.0, 1.0);
        let color = [base_color[0], base_color[1], base_color[2], alpha];

        StickInstance {
            start: self.start.map(|x| x * scale),
            end: self.end.map(|x| x * scale),
            radius: self.thickness_radius * scale,
            color,
        }
    }
}

impl Stylable for Stick {
    fn style_mut(&mut self) -> &mut Material {
        &mut self.style
    }
}

pub trait _UpdateStick {
    fn update_stick(&mut self, id: &str, f: impl FnOnce(&mut Stick));
}

impl _UpdateStick for Scene {
    fn update_stick(&mut self, id: &str, f: impl FnOnce(&mut Stick)) {
        if let Some(Shape::Stick(stick)) = self.named_shapes.get_mut(id) {
            f(stick);
        } else {
            panic!("Stick with ID '{}' not found or is not a Stick", id);
        }
    }
}

fn build_cylinder_body(
    segments: u32,
    r: f32,
    height: f32,
    color: Vec4,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
    colors: &mut Vec<Vec4>,
) {
    let base = vertices.len() as u32;

    for i in 0..=segments {
        let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
        let (cos, sin) = (theta.cos(), theta.sin());

        let x = cos * r;
        let y = sin * r;

        vertices.push(Vec3::new(x, y, 0.0));
        normals.push(Vec3::new(cos, sin, 0.0));
        colors.push(color);

        vertices.push(Vec3::new(x, y, height));
        normals.push(Vec3::new(cos, sin, 0.0));
        colors.push(color);
    }

    for i in 0..segments {
        let i = i as u32;
        let idx = base + i * 2;

        indices.extend_from_slice(&[idx + 2, idx + 1, idx, idx + 2, idx + 3, idx + 1]);
    }
}
fn build_flat_cap(
    z: f32,
    normal: Vec3,
    segments: u32,
    r: f32,
    color: Vec4,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
    colors: &mut Vec<Vec4>,
) {
    let center_idx = vertices.len() as u32;

    vertices.push(Vec3::new(0.0, 0.0, z));
    normals.push(normal);
    colors.push(color);

    for i in 0..=segments {
        let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
        let (cos, sin) = (theta.cos(), theta.sin());

        vertices.push(Vec3::new(cos * r, sin * r, z));
        normals.push(normal);
        colors.push(color);
    }

    for i in 0..segments {
        indices.extend_from_slice(&[center_idx, center_idx + i + 1, center_idx + i + 2]);
    }
}

fn build_round_cap(
    z_base: f32,
    direction: f32, // +1.0 top, -1.0 bottom
    segments: u32,
    rings: u32,
    r: f32,
    color: Vec4,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
    colors: &mut Vec<Vec4>,
) {
    let base = vertices.len() as u32;

    for y in 0..=rings {
        let v = y as f32 / rings as f32;
        let phi = v * std::f32::consts::FRAC_PI_2;

        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        let z = z_base + direction * sin_phi * r;
        let ring_r = cos_phi * r;

        for x in 0..=segments {
            let theta = x as f32 / segments as f32 * std::f32::consts::TAU;
            let (cos, sin) = (theta.cos(), theta.sin());

            let nx = cos * cos_phi;
            let ny = sin * cos_phi;
            let nz = direction * sin_phi;

            vertices.push(Vec3::new(cos * ring_r, sin * ring_r, z));
            normals.push(Vec3::new(nx, ny, nz));
            colors.push(color);
        }
    }

    let row = segments + 1;
    for y in 0..rings {
        for x in 0..segments {
            let i = base + y * row + x;
            indices.extend_from_slice(&[i, i + row, i + 1, i + 1, i + row, i + row + 1]);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StickTemplateId {
    Cylinder { quality: u32, cap: StickCap },
}

struct StickTemplateGpu {
    vao: glow::VertexArray,
    index_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct StickInstance {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
}

impl StickInstance {
    pub fn new(start: [f32; 3], end: [f32; 3], radius: f32, color: [f32; 4]) -> Self {
        Self {
            start,
            end,
            radius,
            color,
        }
    }
}
