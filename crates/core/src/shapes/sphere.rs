use crate::Arc;
use crate::{
    Shape,
    utils::{Interaction, Interpolatable, Logger, Material, MeshData, Stylable},
};
use dashmap::DashMap;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct MeshTemplate {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
}

static SPHERE_TEMPLATE_CACHE: Lazy<DashMap<u32, Arc<MeshTemplate>>> = Lazy::new(|| DashMap::new());

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Sphere {
    pub center: [f32; 3],
    pub radius: f32,
    pub quality: u32,

    pub material: Material,
    pub interaction: Interaction,
}

impl Interpolatable for Sphere {
    fn interpolate(&self, other: &Self, t: f32, _logger: impl Logger) -> Self {
        Self {
            center: [
                self.center[0] * (1.0 - t) + other.center[0] * t,
                self.center[1] * (1.0 - t) + other.center[1] * t,
                self.center[2] * (1.0 - t) + other.center[2] * t,
            ],
            radius: self.radius * (1.0 - t) + other.radius * t,
            quality: ((self.quality as f32) * (1.0 - t) + (other.quality as f32) * t) as u32,
            material: self.material.clone(), // 简单处理，或者给 Material 也实现 interpolate
            interaction: self.interaction.clone(), // 同上
        }
    }
}

impl Into<Shape> for Sphere {
    fn into(self) -> Shape {
        Shape::Sphere(self)
    }
}

impl Sphere {
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self {
            center,
            radius,
            quality: 2,
            material: Material {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
            interaction: Default::default(),
        }
    }

    pub fn set_center(mut self, center: [f32; 3]) -> Self {
        self.center = center;
        self
    }

    pub fn set_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn to_mesh(&self, _scale: f32) -> MeshData {
        return MeshData::default();
    }

    pub fn get_or_generate_sphere_mesh_template(quality: u32) -> Arc<MeshTemplate> {
        if let Some(entry) = SPHERE_TEMPLATE_CACHE.get(&quality) {
            return Arc::clone(entry.value());
        }

        let lat_segments = 10 * quality;
        let lon_segments = 20 * quality;

        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        for i in 0..=lat_segments {
            let theta = std::f32::consts::PI * (i as f32) / (lat_segments as f32);
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            for j in 0..=lon_segments {
                let phi = 2.0 * std::f32::consts::PI * (j as f32) / (lon_segments as f32);
                let sin_phi = phi.sin();
                let cos_phi = phi.cos();

                let nx = sin_theta * cos_phi;
                let ny = cos_theta;
                let nz = sin_theta * sin_phi;

                vertices.push(Vec3::new(nx, ny, nz));
                normals.push(Vec3::new(nx, ny, nz));
            }
        }

        for i in 0..lat_segments {
            for j in 0..lon_segments {
                let first = i * (lon_segments + 1) + j;
                let second = first + lon_segments + 1;

                indices.push(first);
                indices.push(first + 1);
                indices.push(second);

                indices.push(second);
                indices.push(first + 1);
                indices.push(second + 1);
            }
        }

        let template = Arc::new(MeshTemplate {
            vertices,
            normals,
            indices,
        });

        SPHERE_TEMPLATE_CACHE.insert(quality, Arc::clone(&template));

        template
    }

    pub fn get_or_generate_icosphere_mesh_template(quality: u32) -> Arc<MeshTemplate> {
        if let Some(entry) = SPHERE_TEMPLATE_CACHE.get(&quality) {
            return Arc::clone(entry.value());
        }

        let t = (1.0 + 5.0f32.sqrt()) / 2.0;
        let mut vertices = vec![
            Vec3::new(-1.0, t, 0.0),
            Vec3::new(1.0, t, 0.0),
            Vec3::new(-1.0, -t, 0.0),
            Vec3::new(1.0, -t, 0.0),
            Vec3::new(0.0, -1.0, t),
            Vec3::new(0.0, 1.0, t),
            Vec3::new(0.0, -1.0, -t),
            Vec3::new(0.0, 1.0, -t),
            Vec3::new(t, 0.0, -1.0),
            Vec3::new(t, 0.0, 1.0),
            Vec3::new(-t, 0.0, -1.0),
            Vec3::new(-t, 0.0, 1.0),
        ];
        vertices.iter_mut().for_each(|v| *v = v.normalize());

        let mut indices = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        use std::collections::HashMap;
        let mut midpoint_cache: HashMap<(u32, u32), u32> = HashMap::new();

        fn get_midpoint(
            a: u32,
            b: u32,
            vertices: &mut Vec<Vec3>,
            cache: &mut HashMap<(u32, u32), u32>,
        ) -> u32 {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&idx) = cache.get(&key) {
                return idx;
            }

            let midpoint = (vertices[a as usize] + vertices[b as usize]).normalize();
            let idx = vertices.len() as u32;
            vertices.push(midpoint);
            cache.insert(key, idx);
            idx
        }

        for _ in 0..quality {
            let mut new_indices = Vec::new();
            for tri in &indices {
                let a = tri[0];
                let b = tri[1];
                let c = tri[2];

                let ab = get_midpoint(a, b, &mut vertices, &mut midpoint_cache);
                let bc = get_midpoint(b, c, &mut vertices, &mut midpoint_cache);
                let ca = get_midpoint(c, a, &mut vertices, &mut midpoint_cache);

                new_indices.push([a, ab, ca]);
                new_indices.push([b, bc, ab]);
                new_indices.push([c, ca, bc]);
                new_indices.push([ab, bc, ca]);
            }
            indices = new_indices;
        }

        let normals = vertices.clone();
        let mut flat_indices = Vec::with_capacity(indices.len() * 3);
        for tri in indices {
            flat_indices.push(tri[0]);
            flat_indices.push(tri[1]);
            flat_indices.push(tri[2]);
        }

        let template = Arc::new(MeshTemplate {
            vertices,
            normals,
            indices: flat_indices,
        });

        SPHERE_TEMPLATE_CACHE.insert(quality, Arc::clone(&template));
        template
    }

    pub fn to_instance(&self, scale: f32) -> SphereInstance {
        let base_color = self.material.color.unwrap_or([1.0, 1.0, 1.0].into());
        let alpha = self.material.opacity.clamp(0.0, 1.0);
        let color = [base_color[0], base_color[1], base_color[2], alpha];
        let material = [self.material.roughness, self.material.metallic];

        SphereInstance::new(
            self.center.map(|x| x * scale),
            self.radius * scale,
            color,
            material,
        )
    }
}

impl Stylable for Sphere {
    fn style_mut(&mut self) -> &mut Material {
        &mut self.material
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct SphereInstance {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 4],
    pub material: [f32; 2],
}

impl SphereInstance {
    pub fn new(position: [f32; 3], radius: f32, color: [f32; 4], material: [f32; 2]) -> Self {
        Self {
            position,
            radius,
            color,
            material,
        }
    }
}
