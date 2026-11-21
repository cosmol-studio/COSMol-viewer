//! Viewer-specific input declarations. PyO3 derives their unions and extraction.
use crate::shapes::{PySphere, PyStick};
use cosmol_viewer_core::utils::{Color, Shape};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// A shape snapshot. The variants are the single source of the accepted types.
#[derive(FromPyObject)]
pub enum ShapeInput {
    Sphere(PySphere),
    Stick(PyStick),
}

impl From<ShapeInput> for Shape {
    fn from(value: ShapeInput) -> Self {
        match value {
            ShapeInput::Sphere(value) => value.inner.into(),
            ShapeInput::Stick(value) => value.inner.into(),
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
            Self::Hex(value) => Color::from_hex(&value).map_err(PyValueError::new_err),
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
