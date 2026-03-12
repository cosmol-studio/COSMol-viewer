use crate::utils::InstanceGroups;
use crate::utils::Logger;
use glam::Mat3;
use glam::Mat4;
use rayon::prelude::*;
use std::collections::HashMap;
use thiserror::Error;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{
    Shape,
    shader::CameraState,
    utils::{self, Interpolatable, IntoInstanceGroups, ToMesh},
};

// pub enum Instance {
//     Sphere(SphereInstance),
// }
//

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scene {
    pub background_color: [f32; 3],
    pub camera_state: Option<CameraState>,
    pub named_shapes: HashMap<String, Shape>,
    pub unnamed_shapes: Vec<Shape>,
    pub scale: f32,
    pub viewport: Option<[usize; 2]>,
    pub scene_center: [f32; 3],
    pub camera_lights: Option<Lighting>,
    // pub _world_lights: Lighting,
}

#[derive(Error, Debug)]
pub enum SceneError {
    #[error("Shape with ID '{0}' not found")]
    ShapeNotFound(String),
    #[error("Failed to parse scene from JSON")]
    ParseError(#[from] serde_json::Error),
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            background_color: [1.0, 1.0, 1.0],
            camera_state: None,
            named_shapes: HashMap::new(),
            unnamed_shapes: Vec::new(),
            scale: 1.0,
            viewport: None,
            scene_center: [0.0, 0.0, 0.0],
            camera_lights: None,
        }
    }
}

impl Scene {
    pub fn _get_meshes(&self) -> Vec<utils::MeshData> {
        let scale = self.scale;

        let (mut a, mut b) = rayon::join(
            || {
                self.named_shapes
                    .par_iter()
                    .map(|(_, s)| s.to_mesh(scale))
                    .collect::<Vec<_>>()
            },
            || {
                self.unnamed_shapes
                    .par_iter()
                    .map(|s| s.to_mesh(scale))
                    .collect::<Vec<_>>()
            },
        );

        a.append(&mut b);
        a
    }

    pub fn get_instances_grouped(&self) -> InstanceGroups {
        let scale = self.scale;

        // 分别处理两部分
        let (named_groups, unnamed_groups) = rayon::join(
            || {
                self.named_shapes
                    .par_iter()
                    .map(|(_, s)| s.to_instance_group(scale))
                    .reduce(
                        || InstanceGroups::default(),
                        |mut acc, g| {
                            acc.merge(g);
                            acc
                        },
                    )
            },
            || {
                self.unnamed_shapes
                    .par_iter()
                    .map(|s| s.to_instance_group(scale))
                    .reduce(
                        || InstanceGroups::default(),
                        |mut acc, g| {
                            acc.merge(g);
                            acc
                        },
                    )
            },
        );

        let mut result = named_groups;
        result.merge(unnamed_groups);
        result
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn recenter(&mut self, center: [f32; 3]) {
        self.scene_center = center;
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    pub fn add_shape_with_id<S: Into<Shape>>(&mut self, id: impl Into<String>, shape: S) {
        self.named_shapes.insert(id.into(), shape.into());
    }

    pub fn add_shape<S: Into<Shape>>(&mut self, shape: S) {
        self.unnamed_shapes.push(shape.into());
    }

    pub fn replace_shape<S: Into<Shape>>(&mut self, id: &str, shape: S) -> Result<(), SceneError> {
        let shape = shape.into();
        if let Some(existing_shape) = self.named_shapes.get_mut(id) {
            *existing_shape = shape;
            Ok(())
        } else {
            Err(SceneError::ShapeNotFound(id.to_string()))
        }
    }

    pub fn remove_shape(&mut self, id: &str) -> Result<(), SceneError> {
        if self.named_shapes.remove(id).is_none() {
            Err(SceneError::ShapeNotFound(id.to_string()))
        } else {
            Ok(())
        }
    }

    pub fn set_background_color(&mut self, background_color: [f32; 3]) {
        self.background_color = background_color;
    }

    pub fn use_black_background(&mut self) {
        self.background_color = [0.0, 0.0, 0.0];
    }

    /// === u_model ===
    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_translation(-Vec3::from(self.scene_center) * self.scale)
    }

    /// === u_normal_matrix ===
    pub fn normal_matrix(&self) -> Mat3 {
        Mat3::from_mat4(self.model_matrix()).inverse().transpose()
    }

    pub fn merge_shapes(&mut self, other: &Self) {
        self.unnamed_shapes
            .extend(other.named_shapes.iter().map(|(_k, v)| v.clone()));
        self.unnamed_shapes.extend(other.unnamed_shapes.clone());
    }

    pub fn add_camera_light(&mut self, light: Lighting) {
        self.camera_lights = Some(light);
    }

    pub fn add_world_light(&mut self, light: Lighting) {
        println!("{:?}", light);
        unimplemented!()
    }
}

impl Interpolatable for Scene {
    fn interpolate(&self, other: &Self, t: f32, logger: impl Logger) -> Self {
        let named_shapes = self
            .named_shapes
            .iter()
            .filter_map(|(k, v)| {
                other
                    .named_shapes
                    .get(k)
                    .map(|ov| (k.clone(), v.interpolate(ov, t, logger)))
            })
            .collect();

        let unnamed_shapes = self
            .unnamed_shapes
            .iter()
            .zip(&other.unnamed_shapes)
            .map(|(a, b)| a.interpolate(b, t, logger))
            .collect();

        let scene_center =
            Vec3::from(self.scene_center) * (1.0 - t) + Vec3::from(other.scene_center) * t;

        Self {
            background_color: self.background_color,
            camera_state: self.camera_state,
            named_shapes,
            unnamed_shapes,
            scale: self.scale * (1.0 - t) + other.scale * t,
            viewport: self.viewport,
            scene_center: [scene_center.x, scene_center.y, scene_center.z],
            camera_lights: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Animation {
    pub static_scene: Option<Scene>,
    pub frames: Vec<Scene>,
    pub interval: u64,
    pub loops: i64, // -1 = infinite
    pub interpolate: bool,
}

impl Animation {
    pub fn new(interval: f32, loops: i64, interpolate: bool) -> Self {
        Self {
            static_scene: None,
            frames: Vec::new(),
            interval: (interval * 1000.0) as u64,
            loops,
            interpolate,
        }
    }

    pub fn add_frame(&mut self, frame: Scene) {
        self.frames.push(frame);
    }

    pub fn set_static_scene(&mut self, scene: Scene) {
        self.static_scene = Some(scene);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AmbientLight {
    intensity: f32,
    color: Vec3,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub intensity: f32,
    pub color: Vec3,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PointLight {
    position: Vec3,
    intensity: f32,
    color: Vec3,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lighting {
    pub ambient: AmbientLight,
    pub directionals: Option<DirectionalLight>,
    pub points: Option<PointLight>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient: AmbientLight {
                intensity: 0.1,
                color: Vec3::new(1.0, 1.0, 1.0),
            },
            directionals: Some(DirectionalLight {
                direction: Vec3::new(-1.0, 1.0, 5.0) * 1000.0,
                color: Vec3::new(1.0, 0.97, 0.97),
                intensity: 1.0,
            }),
            points: None,
        }
    }
}
