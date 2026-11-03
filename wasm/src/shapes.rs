//! Positioned shapes retain their own material; Scene snapshots them on insertion.
use cosmol_viewer_core::{
    display::SceneItem,
    shapes::{Sphere as CoreSphere, Stick as CoreStick},
};
use wasm_bindgen::prelude::*;
#[derive(Clone)]
#[wasm_bindgen]
pub struct Sphere {
    pub(crate) inner: CoreSphere,
}
#[derive(Clone)]
#[wasm_bindgen]
pub struct Stick {
    pub(crate) inner: CoreStick,
}
impl crate::input::NativeOutput for Sphere {}
impl crate::input::NativeOutput for Stick {}
#[cosmol_viewer_derive::checked_wasm]
impl Sphere {
    pub fn new(center: Vec<f32>, radius: f32) -> Result<Self, String> {
        let inner = CoreSphere::new(crate::scene::vector3(center)?, radius);
        inner.to_shape(None).map_err(|e| e.to_string())?;
        Ok(Self { inner })
    }
    pub fn clone_shape(&self) -> Self {
        self.clone()
    }
    pub fn color(&self, rgb: Vec<u8>) -> Result<Self, String> {
        Ok(Self {
            inner: self.inner.color(crate::scene::rgb(rgb)?),
        })
    }
    pub fn opacity(&self, opacity: f32) -> Result<Self, String> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err("Opacity must be in [0, 1]".into());
        }
        Ok(Self {
            inner: self.inner.opacity(opacity),
        })
    }
}
#[cosmol_viewer_derive::checked_wasm]
impl Stick {
    pub fn new(start: Vec<f32>, end: Vec<f32>, radius: f32) -> Result<Self, String> {
        let inner = CoreStick::new(
            crate::scene::vector3(start)?,
            crate::scene::vector3(end)?,
            radius,
        );
        inner.to_shape(None).map_err(|e| e.to_string())?;
        Ok(Self { inner })
    }
    pub fn clone_shape(&self) -> Self {
        self.clone()
    }
    pub fn color(&self, rgb: Vec<u8>) -> Result<Self, String> {
        Ok(Self {
            inner: self.inner.color(crate::scene::rgb(rgb)?),
        })
    }
    pub fn opacity(&self, opacity: f32) -> Result<Self, String> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err("Opacity must be in [0, 1]".into());
        }
        Ok(Self {
            inner: self.inner.opacity(opacity),
        })
    }
}
