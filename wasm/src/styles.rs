use cosmol_viewer_core::styles::{self as core, Representation};
use wasm_bindgen::prelude::*;
fn error(e: &'static str) -> JsValue {
    js_sys::RangeError::new(e).into()
}
#[wasm_bindgen]
pub struct Style {
    pub(crate) inner: Representation,
}
impl crate::input::NativeOutput for Style {}
#[cosmol_viewer_derive::checked_wasm]
impl Style {
    pub fn clone_style(&self) -> Self {
        Self { inner: self.inner }
    }
    pub fn ball_and_stick() -> Self {
        Self {
            inner: core::BallAndStick::default().into(),
        }
    }
    pub fn sphere() -> Self {
        Self {
            inner: core::Sphere::default().into(),
        }
    }
    pub fn stick() -> Self {
        Self {
            inner: core::Stick::default().into(),
        }
    }
    pub fn cartoon() -> Self {
        Self {
            inner: core::Cartoon::default().into(),
        }
    }
    pub fn surface() -> Self {
        Self {
            inner: core::Surface::default().into(),
        }
    }
    pub fn color(&self, rgb: Vec<u8>) -> Result<Self, JsValue> {
        let rgb: [u8; 3] = rgb
            .try_into()
            .map_err(|_| error("RGB requires three components"))?;
        let mut inner = self.inner;
        inner.set_color(rgb);
        Ok(Self { inner })
    }
    pub fn opacity(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner.set_opacity(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn roughness(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner.set_roughness(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn metallic(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner.set_metallic(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn scale(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner.set_radius_scale(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn outline(&self, rgb: Vec<u8>, width: f32) -> Result<Self, JsValue> {
        let rgb: [u8; 3] = rgb
            .try_into()
            .map_err(|_| error("RGB requires three components"))?;
        let mut inner = self.inner;
        inner.set_outline(rgb, width).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn coordinates_2d(&self) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner
            .set_coordinates(core::Coordinates::TwoD)
            .map_err(error)?;
        Ok(Self { inner })
    }
    pub fn conformer(&self, id: usize) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner
            .set_coordinates(core::Coordinates::ThreeD { id })
            .map_err(error)?;
        Ok(Self { inner })
    }
    pub fn rainbow_residues(&self) -> Result<Self, JsValue> {
        let mut inner = self.inner;
        inner.set_rainbow_residues().map_err(error)?;
        Ok(Self { inner })
    }
    pub fn configure_surface(
        &self,
        probe_radius: f32,
        grid_spacing: f32,
        solvent_accessible: bool,
        sharp_boundaries: bool,
    ) -> Result<Self, JsValue> {
        if !probe_radius.is_finite()
            || probe_radius <= 0.
            || !grid_spacing.is_finite()
            || grid_spacing <= 0.
        {
            return Err(error(
                "surface radii and spacing must be finite and positive",
            ));
        }
        match self.inner {
            Representation::Surface(mut s) => {
                s.probe_radius = probe_radius;
                s.grid_spacing = grid_spacing;
                s.solvent_accessible = solvent_accessible;
                s.sharp_boundaries = sharp_boundaries;
                Ok(Self { inner: s.into() })
            }
            _ => Err(error("surface settings require a Surface style")),
        }
    }
}
