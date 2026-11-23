//! Shared shape-style projection over the native Stylable trait.
macro_rules! impl_stylable {
    ($wrapper:ident) => {
        #[pyo3::pymethods]
        impl $wrapper {
            /// Override scene silhouettes with a width in output pixels.
            pub fn silhouettes(
                mut slf: pyo3::PyRefMut<'_, Self>,
                color: $crate::input::ColorInput,
                width: f32,
            ) -> pyo3::PyResult<pyo3::PyRefMut<'_, Self>> {
                use cosmol_viewer_core::scene::Silhouettes;
                slf.silhouettes = Some(
                    Silhouettes::for_shape(color.into_color()?, width)
                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?,
                );
                Ok(slf)
            }

            /// Set RGB/hex color and return this shape for chaining.
            pub fn color<'py>(
                mut slf: pyo3::PyRefMut<'py, Self>,
                color: $crate::input::ColorInput,
            ) -> pyo3::PyResult<pyo3::PyRefMut<'py, Self>> {
                use cosmol_viewer_core::utils::Stylable as _;
                let color = color.into_color()?;
                slf.inner.style_mut().color = Some(color.into());
                Ok(slf)
            }

            /// Set surface opacity and return this shape for chaining.
            pub fn opacity(
                mut slf: pyo3::PyRefMut<'_, Self>,
                opacity: f32,
            ) -> pyo3::PyRefMut<'_, Self> {
                use cosmol_viewer_core::utils::Stylable as _;
                slf.inner.style_mut().opacity = opacity;
                slf
            }

            /// Set surface roughness and return this shape for chaining.
            pub fn roughness(
                mut slf: pyo3::PyRefMut<'_, Self>,
                roughness: f32,
            ) -> pyo3::PyRefMut<'_, Self> {
                use cosmol_viewer_core::utils::Stylable as _;
                slf.inner.style_mut().roughness = roughness;
                slf
            }

            /// Set the metallic factor and return this shape for chaining.
            pub fn metallic(
                mut slf: pyo3::PyRefMut<'_, Self>,
                metallic: f32,
            ) -> pyo3::PyRefMut<'_, Self> {
                use cosmol_viewer_core::utils::Stylable as _;
                slf.inner.style_mut().metallic = metallic;
                slf
            }
        }
    };
}
pub(crate) use impl_stylable;
