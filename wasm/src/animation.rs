//! Browser animation snapshots over the core model.
use crate::Scene;
use cosmol_viewer_core::scene::Animation as CoreAnimation;
use std::sync::{Arc, Mutex};
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
use wasm_bindgen::prelude::wasm_bindgen;

/// Reusable animation state. Added frames are scene snapshots, matching core behavior.
#[derive(Clone)]
#[cfg_attr(all(feature = "wasm", target_arch = "wasm32"), wasm_bindgen)]
pub struct Animation {
    state: Arc<Mutex<CoreAnimation>>,
}

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
impl crate::input::NativeOutput for Animation {}
#[cfg_attr(
    all(feature = "wasm", target_arch = "wasm32"),
    cosmol_viewer_derive::checked_wasm
)]
impl Animation {
    pub fn new(interval: f32, loops: i64, interpolate: bool) -> Self {
        CoreAnimation::new(interval, loops, interpolate).into()
    }
    pub fn add_frame(&self, scene: &Scene) {
        self.state.lock().unwrap().add_frame(scene.snapshot());
    }
    pub fn set_static_scene(&self, scene: &Scene) {
        self.state
            .lock()
            .unwrap()
            .set_static_scene(scene.snapshot());
    }
    pub fn set_interval(&self, interval: f32) {
        self.state.lock().unwrap().set_interval(interval);
    }
    pub fn set_loops(&self, loops: i64) {
        self.state.lock().unwrap().set_loops(loops);
    }
    pub fn set_interpolate(&self, interpolate: bool) {
        self.state.lock().unwrap().set_interpolate(interpolate);
    }
    pub fn frame_count(&self) -> usize {
        self.state.lock().unwrap().frames.len()
    }
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&*self.state.lock().unwrap()).map_err(|error| error.to_string())
    }
}

impl Animation {
    pub(crate) fn snapshot_for_render(&self) -> CoreAnimation {
        let mut animation = self.state.lock().unwrap().clone();
        if let Some(scene) = animation.static_scene.as_mut() {
            scene.prepare_for_wasm();
        }
        for scene in &mut animation.frames {
            scene.prepare_for_wasm();
        }
        animation
    }
}
impl From<CoreAnimation> for Animation {
    fn from(animation: CoreAnimation) -> Self {
        Self {
            state: Arc::new(Mutex::new(animation)),
        }
    }
}
