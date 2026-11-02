//! Browser handles over the existing core scene/animation models, not transport payloads.
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
use wasm_bindgen::prelude::wasm_bindgen;

use cosmol_viewer_core::{
    scene::Scene as CoreScene,
    shapes::{Sphere, Stick},
};
use std::sync::{Arc, Mutex};

/// Reusable scene state. Cloned binding handles refer to the same scene.
#[derive(Clone)]
#[cfg_attr(all(feature = "wasm", target_arch = "wasm32"), wasm_bindgen)]
pub struct Scene {
    state: Arc<Mutex<CoreScene>>,
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
impl crate::input::NativeOutput for Scene {}

#[cfg_attr(
    all(feature = "wasm", target_arch = "wasm32"),
    cosmol_viewer_derive::checked_wasm
)]
impl Scene {
    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    pub fn show(
        &self,
        source: &crate::selection::ScientificInput,
        selection: &crate::selection::Selection,
        style: &crate::styles::Style,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let display = selection.project(source, style.inner)?;
        self.state
            .lock()
            .unwrap()
            .add_shape(cosmol_viewer_core::utils::Shape::Display(display));
        Ok(())
    }
    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    pub fn show_with_id(
        &self,
        id: String,
        source: &crate::selection::ScientificInput,
        selection: &crate::selection::Selection,
        style: &crate::styles::Style,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let display = selection.project(source, style.inner)?;
        self.state
            .lock()
            .unwrap()
            .add_shape_with_id(id, cosmol_viewer_core::utils::Shape::Display(display));
        Ok(())
    }
    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    pub fn recenter_on(
        &self,
        source: &crate::selection::ScientificInput,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let center = source
            .capture()?
            .center()
            .map_err(|e| js_sys::Error::new(&e.to_string()))?;
        self.state.lock().unwrap().recenter(center);
        Ok(())
    }
    pub fn new() -> Self {
        CoreScene::new().into()
    }
    /// Explicitly copy scene display state into an independent scene.
    pub fn clone_scene(&self) -> Self {
        self.snapshot().into()
    }
    pub fn recenter(&self, center: Vec<f32>) -> Result<(), String> {
        self.state.lock().unwrap().recenter(vector3(center)?);
        Ok(())
    }
    pub fn set_scale(&self, scale: f32) -> Result<(), String> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err("Scale must be finite and positive".into());
        }
        self.state.lock().unwrap().set_scale(scale);
        Ok(())
    }
    /// RGB components are bytes in the range 0..255.
    pub fn set_background_color(&self, color: Vec<u8>) -> Result<(), String> {
        self.state.lock().unwrap().set_background_color(rgb(color)?);
        Ok(())
    }
    pub fn set_transparent_background(&self, enabled: bool) {
        self.state
            .lock()
            .unwrap()
            .set_transparent_background(enabled);
    }
    pub fn set_zoom_disabled(&self, disabled: bool) {
        self.state.lock().unwrap().set_zoom_disabled(disabled);
    }
    pub fn set_auto_rotate(&self, enabled: bool, speed: f32) {
        self.state.lock().unwrap().set_auto_rotate(enabled, speed);
    }
    pub fn set_depth_cue(&self, enabled: bool) {
        self.state.lock().unwrap().set_depth_cue(enabled);
    }
    pub fn set_depth_cue_range(&self, start: f32, end: f32) -> Result<(), String> {
        if !(0.0..=1.0).contains(&start) || !(0.0..=1.0).contains(&end) || start >= end {
            return Err("Depth cue range must satisfy 0 <= start < end <= 1".into());
        }
        self.state.lock().unwrap().set_depth_cue_range(start, end);
        Ok(())
    }
    pub fn set_depth_cue_color(&self, color: Vec<u8>) -> Result<(), String> {
        self.state.lock().unwrap().set_depth_cue_color(rgb(color)?);
        Ok(())
    }
    pub fn use_black_background(&self) {
        self.state.lock().unwrap().use_black_background();
    }
    pub fn set_camera_view(
        &self,
        azimuth: f32,
        elevation: f32,
        roll: f32,
        distance: f32,
        target: Vec<f32>,
        fov: f32,
    ) -> Result<(), String> {
        self.state.lock().unwrap().set_camera_view(
            azimuth,
            elevation,
            roll,
            distance,
            vector3(target)?,
            fov,
        );
        Ok(())
    }
    pub fn rotate_camera(&self, azimuth_delta: f32, elevation_delta: f32, roll_delta: f32) {
        self.state
            .lock()
            .unwrap()
            .rotate_camera(azimuth_delta, elevation_delta, roll_delta);
    }
    pub fn set_camera_distance(&self, distance: f32) {
        self.state.lock().unwrap().set_camera_distance(distance);
    }
    pub fn set_camera_target(&self, target: Vec<f32>) -> Result<(), String> {
        self.state
            .lock()
            .unwrap()
            .set_camera_target(vector3(target)?);
        Ok(())
    }
    pub fn set_camera_fov(&self, fov: f32) {
        self.state.lock().unwrap().set_camera_fov(fov);
    }
    pub fn set_lighting(
        &self,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        intensity: f32,
        color: Vec<u8>,
    ) -> Result<(), String> {
        let color = cosmol_viewer_core::utils::Color::from(rgb(color)?).0;
        self.state
            .lock()
            .unwrap()
            .set_lighting(ambient, diffuse, specular, intensity, color);
        Ok(())
    }
    pub fn set_light_intensity(&self, intensity: f32) {
        self.state.lock().unwrap().set_light_intensity(intensity);
    }
    pub fn set_ambient_light(&self, intensity: f32) {
        self.state.lock().unwrap().set_ambient_light(intensity);
    }
    pub fn set_light_color(&self, color: Vec<u8>) -> Result<(), String> {
        self.state.lock().unwrap().set_light_color(rgb(color)?);
        Ok(())
    }
    /// Add or replace a named sphere directly in this scene.
    pub fn add_sphere(&self, id: String, center: Vec<f32>, radius: f32) -> Result<(), String> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err("Radius must be finite and positive".into());
        }
        let shape = Sphere::new(vector3(center)?, radius);
        self.state.lock().unwrap().add_shape_with_id(id, shape);
        Ok(())
    }
    pub fn add_stick(
        &self,
        id: String,
        start: Vec<f32>,
        end: Vec<f32>,
        radius: f32,
    ) -> Result<(), String> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err("Radius must be finite and positive".into());
        }
        let shape = Stick::new(vector3(start)?, vector3(end)?, radius);
        self.state.lock().unwrap().add_shape_with_id(id, shape);
        Ok(())
    }
    pub fn remove_shape(&self, id: String) -> Result<(), String> {
        self.state
            .lock()
            .unwrap()
            .remove_shape(&id)
            .map_err(|error| error.to_string())
    }
    pub fn merge_shapes(&self, other: &Scene) {
        // Snapshot first: merging a scene with itself must not lock it twice.
        let other = other.snapshot();
        self.state.lock().unwrap().merge_shapes(&other);
    }
    pub fn shape_count(&self) -> usize {
        let scene = self.state.lock().unwrap();
        scene.named_shapes.len() + scene.unnamed_shapes.len()
    }
    /// Inspect display state; this serialization is not needed for render/update.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&*self.state.lock().unwrap()).map_err(|error| error.to_string())
    }
}

impl Scene {
    pub(crate) fn snapshot(&self) -> CoreScene {
        self.state.lock().unwrap().clone()
    }
    pub(crate) fn snapshot_for_render(&self) -> CoreScene {
        let mut scene = self.snapshot();
        scene.prepare_for_wasm();
        scene
    }
}
impl From<CoreScene> for Scene {
    fn from(scene: CoreScene) -> Self {
        Self {
            state: Arc::new(Mutex::new(scene)),
        }
    }
}
impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

fn vector3(values: Vec<f32>) -> Result<[f32; 3], String> {
    let values: [f32; 3] = values
        .try_into()
        .map_err(|_| "Expected three coordinates".to_string())?;
    if !values.iter().all(|value| value.is_finite()) {
        return Err("Coordinates must be finite".into());
    }
    Ok(values)
}
fn rgb(values: Vec<u8>) -> Result<[u8; 3], String> {
    values
        .try_into()
        .map_err(|_| "Expected three RGB components".into())
}
