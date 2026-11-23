use crate::input::{BytesInput, CheckedValue};
use cosmol_viewer_core::styles::{self as core, Representation};
use cosmol_viewer_core::utils::Color;
use serde::Deserialize;
use std::collections::HashMap;
use tsify::Tsify;
use wasm_bindgen::{JsCast, prelude::*};

fn error(e: &'static str) -> JsValue {
    js_sys::RangeError::new(e).into()
}

#[derive(Deserialize, Tsify)]
#[serde(transparent)]
#[tsify(hashmap_as_object)]
pub struct ColorEntries(HashMap<String, BytesInput>);

impl CheckedValue for ColorEntries {
    type Wire = Self;

    fn unwrap(value: Self) -> Self {
        value
    }

    fn check_representation(value: &JsValue) -> Result<(), JsValue> {
        if !value.is_object()
            || js_sys::Array::is_array(value)
            || js_sys::ArrayBuffer::is_view(value)
            || value.is_instance_of::<js_sys::Map>()
        {
            return Err(
                js_sys::TypeError::new("expected an object mapping keys to RGB bytes").into(),
            );
        }
        for entry in js_sys::Object::entries(value.unchecked_ref()).iter() {
            let entry = entry.unchecked_into::<js_sys::Array>();
            <Vec<u8> as CheckedValue>::check_representation(&entry.get(1))?;
        }
        Ok(())
    }
}

impl ColorEntries {
    fn into_colors(self) -> impl Iterator<Item = Result<(String, Color), JsValue>> {
        self.0.into_iter().map(|(key, color)| {
            let rgb = crate::scene::rgb(<Vec<u8> as CheckedValue>::unwrap(color))
                .map_err(|e| js_sys::RangeError::new(&e))?;
            Ok((key, Color::from(rgb)))
        })
    }
}

/// Reusable element and per-atom colors for molecular displays.
#[wasm_bindgen]
pub struct MoleculeColors {
    pub(crate) inner: core::MoleculeColors,
}

impl crate::input::NativeOutput for MoleculeColors {}

#[cosmol_viewer_derive::checked_wasm]
impl MoleculeColors {
    pub fn new(elements: ColorEntries) -> Result<Self, JsValue> {
        let elements = elements.into_colors().collect::<Result<Vec<_>, _>>()?;
        let inner = core::MoleculeColors::new(elements)
            .map_err(|e| js_sys::RangeError::new(&e.to_string()))?;
        Ok(Self { inner })
    }

    /// Return the built-in molecular color scheme.
    pub fn default() -> Self {
        Self {
            inner: core::MoleculeColors::default(),
        }
    }

    /// Return a new scheme with colors keyed by zero-based CK atom ID.
    pub fn with_atom_colors(&self, atoms: ColorEntries) -> Result<Self, JsValue> {
        let atoms = atoms
            .into_colors()
            .map(|entry| {
                let (key, color) = entry?;
                let id = key.parse::<usize>().map_err(|_| {
                    js_sys::RangeError::new(
                        &core::MoleculeColorError::InvalidAtomId(key).to_string(),
                    )
                })?;
                Ok((id, color))
            })
            .collect::<Result<Vec<_>, JsValue>>()?;
        let inner = self
            .inner
            .with_atom_colors(atoms)
            .map_err(|e| js_sys::RangeError::new(&e.to_string()))?;
        Ok(Self { inner })
    }
}

#[wasm_bindgen]
pub struct Style {
    pub(crate) inner: core::DisplayStyle,
}

#[wasm_bindgen]
impl Style {
    /// Apply a reusable molecular color scheme.
    pub fn colors(&self, colors: &MoleculeColors) -> Result<Style, JsValue> {
        let mut inner = self.inner.clone();
        inner.set_molecule_colors(&colors.inner).map_err(error)?;
        Ok(Self { inner })
    }
}
impl crate::input::NativeOutput for Style {}
#[cosmol_viewer_derive::checked_wasm]
impl Style {
    pub fn clone_style(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
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
        let mut inner = self.inner.clone();
        inner.representation.set_color(rgb);
        Ok(Self { inner })
    }
    pub fn opacity(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner.representation.set_opacity(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn roughness(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner.representation.set_roughness(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn metallic(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner.representation.set_metallic(value).map_err(error)?;
        Ok(Self { inner })
    }
    pub fn scale(&self, value: f32) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner
            .representation
            .set_radius_scale(value)
            .map_err(error)?;
        Ok(Self { inner })
    }
    /// When disabled, draw one stick per bond regardless of its order.
    pub fn show_bond_order(&self, show: bool) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner
            .representation
            .set_show_bond_order(show)
            .map_err(error)?;
        Ok(Self { inner })
    }
    /// Override scene silhouettes. Width is in pixels. Zero disables this shape's edges.
    pub fn silhouettes(&self, rgb: Vec<u8>, width: f32) -> Result<Self, JsValue> {
        let rgb: [u8; 3] = rgb
            .try_into()
            .map_err(|_| error("RGB requires three components"))?;
        let mut inner = self.inner.clone();
        inner
            .representation
            .set_silhouettes(rgb, width)
            .map_err(|e| js_sys::RangeError::new(&e.to_string()))?;
        Ok(Self { inner })
    }
    pub fn coordinates_2d(&self) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner
            .representation
            .set_coordinates(core::Coordinates::TwoD)
            .map_err(error)?;
        Ok(Self { inner })
    }
    pub fn conformer(&self, id: usize) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner
            .representation
            .set_coordinates(core::Coordinates::ThreeD { id })
            .map_err(error)?;
        Ok(Self { inner })
    }
    pub fn rainbow_residues(&self) -> Result<Self, JsValue> {
        let mut inner = self.inner.clone();
        inner.representation.set_rainbow_residues().map_err(error)?;
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
        match self.inner.representation {
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
