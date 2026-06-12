use crate::utils::Color;
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
    pub background_color: Vec3,
    #[serde(default)]
    pub transparent_background: bool,
    #[serde(default)]
    pub zoom_disabled: bool,
    #[serde(default)]
    pub auto_rotate: AutoRotate,
    #[serde(default)]
    pub depth_cue: DepthCue,
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
            background_color: Vec3::ONE,
            transparent_background: false,
            zoom_disabled: false,
            auto_rotate: AutoRotate::default(),
            depth_cue: DepthCue::default(),
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
        let shape_count = self.named_shapes.len() + self.unnamed_shapes.len();

        if shape_count <= 128 {
            let mut groups = InstanceGroups::default();

            for shape in self.named_shapes.values() {
                groups.merge(shape.to_instance_group(scale));
            }

            for shape in &self.unnamed_shapes {
                groups.merge(shape.to_instance_group(scale));
            }

            return groups;
        }

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

    pub fn set_background_color<C: Into<Color>>(&mut self, background_color: C) {
        self.background_color = background_color.into().into()
    }

    pub fn set_transparent_background(&mut self, enabled: bool) {
        self.transparent_background = enabled;
    }

    pub fn set_zoom_disabled(&mut self, disabled: bool) {
        self.zoom_disabled = disabled;
    }

    pub fn set_auto_rotate(&mut self, enabled: bool, speed: f32) {
        self.auto_rotate = AutoRotate { enabled, speed };
    }

    /// Enable or disable depth cueing for the whole scene.
    ///
    /// Depth cueing follows the ChimeraX model: fragment color is linearly
    /// mixed toward the cue color according to camera-space depth. By default,
    /// depth cueing is disabled. When enabled, the default cue color follows the
    /// scene background color, so distant geometry fades into the background. It
    /// applies to all standard rendered geometry, including molecules, sticks,
    /// spheres, ribbons, and protein surfaces.
    pub fn set_depth_cue(&mut self, enabled: bool) {
        self.depth_cue.enabled = enabled;
    }

    /// Set the fractional depth cue range.
    ///
    /// `start` and `end` are fractions of the current scene depth range. Dimming
    /// begins at `start` and reaches the cue color at `end`. The default range is
    /// `0.5..1.0`, matching ChimeraX defaults. The range is only used when depth
    /// cueing is enabled with [`set_depth_cue`](Self::set_depth_cue).
    ///
    /// # Panics
    ///
    /// Panics if `start` and `end` do not satisfy `0.0 <= start < end <= 1.0`.
    pub fn set_depth_cue_range(&mut self, start: f32, end: f32) {
        if !(0.0..=1.0).contains(&start) || !(0.0..=1.0).contains(&end) || start >= end {
            panic!("depth cue range must satisfy 0.0 <= start < end <= 1.0");
        }
        self.depth_cue.start = start;
        self.depth_cue.end = end;
    }

    /// Set an explicit depth cue color.
    ///
    /// If this is not called, depth cueing fades geometry toward the scene
    /// background color. Use this method to override that behavior, for example
    /// to fade toward black on a non-black background.
    pub fn set_depth_cue_color<C: Into<Color>>(&mut self, color: C) {
        self.depth_cue.color = Some(color.into().into());
    }

    pub fn use_black_background(&mut self) {
        self.background_color = Vec3::ZERO;
    }

    pub fn set_camera_view(
        &mut self,
        azimuth: f32,
        elevation: f32,
        roll: f32,
        distance: f32,
        target: [f32; 3],
        fov: f32,
    ) {
        self.camera_state = Some(CameraState::from_orbit_angles(
            azimuth, elevation, roll, distance, target, fov,
        ));
    }

    pub fn rotate_camera(&mut self, azimuth_delta: f32, elevation_delta: f32, roll_delta: f32) {
        let mut camera_state = self.camera_state.unwrap_or_default();
        camera_state.rotate_by_degrees(azimuth_delta, elevation_delta, roll_delta);
        self.camera_state = Some(camera_state);
    }

    pub fn set_camera_distance(&mut self, distance: f32) {
        let mut camera_state = self.camera_state.unwrap_or_default();
        camera_state.distance = distance;
        self.camera_state = Some(camera_state);
    }

    pub fn set_camera_target(&mut self, target: [f32; 3]) {
        let mut camera_state = self.camera_state.unwrap_or_default();
        camera_state.target = Vec3::from(target);
        self.camera_state = Some(camera_state);
    }

    pub fn set_camera_fov(&mut self, fov: f32) {
        let mut camera_state = self.camera_state.unwrap_or_default();
        camera_state.fov = fov;
        self.camera_state = Some(camera_state);
    }

    pub fn prepare_for_wasm(&mut self) {
        for shape in self.named_shapes.values_mut() {
            if let Shape::Protein(protein) = shape {
                protein.init_secondary_structure();
            }
        }

        for shape in &mut self.unnamed_shapes {
            if let Shape::Protein(protein) = shape {
                protein.init_secondary_structure();
            }
        }
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
            transparent_background: self.transparent_background,
            zoom_disabled: self.zoom_disabled,
            auto_rotate: self.auto_rotate,
            depth_cue: self.depth_cue,
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct AutoRotate {
    pub enabled: bool,
    pub speed: f32,
}

impl Default for AutoRotate {
    fn default() -> Self {
        Self {
            enabled: false,
            speed: 20.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct DepthCue {
    pub enabled: bool,
    pub start: f32,
    pub end: f32,
    #[serde(default)]
    pub color: Option<Vec3>,
}

impl Default for DepthCue {
    fn default() -> Self {
        Self {
            enabled: false,
            start: 0.5,
            end: 1.0,
            color: None,
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
