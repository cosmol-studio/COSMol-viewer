//! Viewer-specific input declarations. PyO3 derives their unions and extraction.
use crate::shapes::{PyMolecule, PyProtein, PySphere, PyStick};
use cosmol_viewer_core::utils::{Color, Shape};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// A shape snapshot. The variants are the single source of the accepted types.
#[derive(FromPyObject)]
pub enum ShapeInput {
    Sphere(PySphere),
    Stick(PyStick),
    Molecule(PyMolecule),
    Protein(PyProtein),
}

impl From<ShapeInput> for Shape {
    fn from(value: ShapeInput) -> Self {
        match value {
            ShapeInput::Sphere(value) => value.inner.into(),
            ShapeInput::Stick(value) => value.inner.into(),
            ShapeInput::Molecule(value) => value.inner.into(),
            ShapeInput::Protein(value) => value.inner.into(),
        }
    }
}

/// Hex strings or RGB byte sequences. Validation completes before assignment.
#[derive(FromPyObject)]
pub enum ColorInput {
    Hex(String),
    Rgb([i64; 3]),
}

impl ColorInput {
    pub(crate) fn into_color(self) -> PyResult<Color> {
        match self {
            Self::Hex(value) => {
                let hex = value.strip_prefix('#').unwrap_or(&value);
                if hex.len() != 6 || !hex.is_ascii() {
                    return Err(PyValueError::new_err(
                        "Hex color must contain six ASCII hex digits",
                    ));
                }
                let mut rgb = [0; 3];
                for (index, output) in rgb.iter_mut().enumerate() {
                    *output = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                        .map_err(|_| PyValueError::new_err("Invalid hex color"))?;
                }
                Ok(Color::from(rgb))
            }
            Self::Rgb(values) => {
                let mut rgb = [0; 3];
                for (output, input) in rgb.iter_mut().zip(values) {
                    *output = u8::try_from(input).map_err(|_| {
                        PyValueError::new_err(format!("Color value {input} out of range [0, 255]"))
                    })?;
                }
                Ok(Color::from(rgb))
            }
        }
    }
}
