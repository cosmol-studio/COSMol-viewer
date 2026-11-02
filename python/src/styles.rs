use crate::input::ColorInput;
use cosmol_viewer_core::styles as core;
use pyo3::{exceptions::PyValueError, prelude::*};
fn error(e: &'static str) -> PyErr {
    PyValueError::new_err(e)
}
macro_rules! style {
    ($name:ident,$native:ident) => {
        #[derive(Clone)]
        #[pyclass(module = "cosmol_viewer", from_py_object)]
        pub struct $name {
            pub(crate) inner: core::Representation,
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
                slf.inner.set_color(color.into_color()?);
                Ok(slf)
            }
            pub fn opacity(
                mut slf: PyRefMut<'_, Self>,
                value: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner.set_opacity(value).map_err(error)?;
                Ok(slf)
            }
            pub fn roughness(
                mut slf: PyRefMut<'_, Self>,
                value: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner.set_roughness(value).map_err(error)?;
                Ok(slf)
            }
            pub fn metallic(
                mut slf: PyRefMut<'_, Self>,
                value: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner.set_metallic(value).map_err(error)?;
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
            pub fn scale(mut slf: PyRefMut<'_, Self>, value: f32) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner.set_radius_scale(value).map_err(error)?;
                Ok(slf)
            }
            pub fn outline(
                mut slf: PyRefMut<'_, Self>,
                color: ColorInput,
                width: f32,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .set_outline(color.into_color()?, width)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn coordinates_2d(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
                    .set_coordinates(core::Coordinates::TwoD)
                    .map_err(error)?;
                Ok(slf)
            }
            pub fn conformer(
                mut slf: PyRefMut<'_, Self>,
                id: usize,
            ) -> PyResult<PyRefMut<'_, Self>> {
                slf.inner
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
        slf.inner.set_rainbow_residues().map_err(error)?;
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
        if let core::Representation::Surface(ref mut s) = slf.inner {
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
    pub(crate) fn representation(self) -> core::Representation {
        match self {
            Self::BallAndStick(s) => s.inner,
            Self::Sphere(s) => s.inner,
            Self::Stick(s) => s.inner,
            Self::Cartoon(s) => s.inner,
            Self::Surface(s) => s.inner,
        }
    }
}
