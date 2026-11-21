use crate::input::ColorInput;
use cosmol_viewer_core::styles as core;
use cosmol_viewer_core::utils::Color;
use pyo3::{exceptions::PyValueError, prelude::*};
use std::collections::HashMap;

/// Reusable element and per-atom colors for molecular displays.
#[pyclass(module = "cosmol_viewer")]
pub struct MoleculeColors {
    pub(crate) inner: core::MoleculeColors,
}

fn read_colors<K>(colors: HashMap<K, ColorInput>) -> PyResult<Vec<(K, Color)>> {
    colors
        .into_iter()
        .map(|(key, color)| Ok((key, color.into_color()?)))
        .collect()
}

#[pymethods]
impl MoleculeColors {
    #[new]
    #[pyo3(signature = (elements=None))]
    pub fn new(elements: Option<HashMap<String, ColorInput>>) -> PyResult<Self> {
        let inner = core::MoleculeColors::new(read_colors(elements.unwrap_or_default())?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Return the built-in molecular color scheme.
    #[staticmethod]
    pub fn default() -> Self {
        Self {
            inner: core::MoleculeColors::default(),
        }
    }

    /// Return a new scheme with colors keyed by zero-based CK atom ID.
    pub fn with_atom_colors(&self, atoms: HashMap<usize, ColorInput>) -> PyResult<Self> {
        let inner = self
            .inner
            .with_atom_colors(read_colors(atoms)?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }
}

fn error(e: &'static str) -> PyErr {
    PyValueError::new_err(e)
}
macro_rules! style {
    ($name:ident,$native:ident) => {
        #[derive(Clone)]
        #[pyclass(module = "cosmol_viewer", from_py_object)]
        pub struct $name {
            inner: core::DisplayStyle,
        }
        #[pymethods]
        impl $name {
            #[new]
            pub fn new() -> Self {
                Self {
                    inner: core::$native::default().into(),
                }
            }
            pub fn color(
                mut slf: PyRefMut<'_, Self>,
                color: ColorInput,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner.representation.set_color(color.into_color()?);
                Ok(slf)
            }
            pub fn opacity(
                mut slf: PyRefMut<'_, Self>,
                value: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner.representation.set_opacity(value).map_err(error)?;
                Ok(slf)
            }
            pub fn roughness(
                mut slf: PyRefMut<'_, Self>,
                value: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .representation
                    .set_roughness(value)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn metallic(
                mut slf: PyRefMut<'_, Self>,
                value: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .representation
                    .set_metallic(value)
                    .map_err(error)?;
                Ok(slf)
            }
        }
    };
}
style!(BallAndStick, BallAndStick);
style!(SphereStyle, Sphere);
style!(StickStyle, Stick);
style!(Cartoon, Cartoon);
style!(Surface, Surface);
macro_rules! glyph {
    ($name:ident) => {
        #[pymethods]
        impl $name {
            /// Apply a reusable molecular color scheme.
            pub fn colors<'py>(
                mut slf: PyRefMut<'py, Self>,
                colors: &MoleculeColors,
            ) -> PyResult<PyRefMut<'py, Self>> {
                slf.inner
                    .set_molecule_colors(&colors.inner)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn scale(mut slf: PyRefMut<'_, Self>, value: f32) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .representation
                    .set_radius_scale(value)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn outline(
                mut slf: PyRefMut<'_, Self>,
                color: ColorInput,
                width: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .representation
                    .set_outline(color.into_color()?, width)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn coordinates_2d(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .representation
                    .set_coordinates(core::Coordinates::TwoD)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn conformer(
                mut slf: PyRefMut<'_, Self>,
                id: usize,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .representation
                    .set_coordinates(core::Coordinates::ThreeD { id })
                    .map_err(error)?;
                Ok(slf)
            }
        }
    };
}
glyph!(BallAndStick);
glyph!(SphereStyle);
glyph!(StickStyle);
#[pymethods]
impl Cartoon {
    pub fn rainbow_residues(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        slf.inner
            .representation
            .set_rainbow_residues()
            .map_err(error)?;
        Ok(slf)
    }
}
#[pymethods]
impl Surface {
    #[pyo3(signature=(probe_radius=1.4,grid_spacing=0.5,solvent_accessible=false,sharp_boundaries=true))]
    pub fn configure(
        mut slf: PyRefMut<'_, Self>,
        probe_radius: f32,
        grid_spacing: f32,
        solvent_accessible: bool,
        sharp_boundaries: bool,
    ) -> PyResult<PyRefMut<'_, Self>> {
        if !probe_radius.is_finite()
            || probe_radius <= 0.
            || !grid_spacing.is_finite()
            || grid_spacing <= 0.
        {
            return Err(PyValueError::new_err(
                "surface radii and spacing must be finite and positive",
            ));
        }
        if let core::Representation::Surface(ref mut s) = slf.inner.representation {
            s.probe_radius = probe_radius;
            s.grid_spacing = grid_spacing;
            s.solvent_accessible = solvent_accessible;
            s.sharp_boundaries = sharp_boundaries;
        }
        Ok(slf)
    }
}
#[derive(FromPyObject)]
pub enum StyleInput {
    BallAndStick(BallAndStick),
    Sphere(SphereStyle),
    Stick(StickStyle),
    Cartoon(Cartoon),
    Surface(Surface),
}
impl StyleInput {
    pub(crate) fn display_style(self) -> core::DisplayStyle {
        match self {
            Self::BallAndStick(s) => s.inner,
            Self::Sphere(s) => s.inner,
            Self::Stick(s) => s.inner,
            Self::Cartoon(s) => s.inner,
            Self::Surface(s) => s.inner,
        }
    }
}
