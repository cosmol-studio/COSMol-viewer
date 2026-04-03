use pyo3_stub_gen::inventory::submit;

#[macro_export]
macro_rules! impl_stylable_pymethods {
    ($pyclass:ident, $return_ty:ident) => {
        #[gen_stub_pymethods]
        #[pymethods]
        impl $pyclass {
            #[gen_stub(skip)]
            #[doc = concat!(
                                                        "color(self, c: tuple[int, int, int]) -> ",
                                                        stringify!($return_ty),
                                                        "\n",
                                                        "color(self, c: str) -> ",
                                                        stringify!($return_ty),
                                                        "\n\n",
                                                        "Set the shape color.\n"
                                                    )]
            pub fn color<'py>(
                mut slf: PyRefMut<'py, Self>,
                color: Bound<'py, PyAny>,
            ) -> PyResult<PyRefMut<'py, Self>> {
                let color = py_to_color(color)?;
                slf.inner.style_mut().color = Some(color.into());
                Ok(slf)
            }

            #[doc = concat!(
                                                        "opacity(self, opacity: float) -> ",
                                                        stringify!($return_ty),
                                                        "\n\n",
                                                        "Set the surface opacity.\n"
                                                    )]
            pub fn opacity(mut slf: PyRefMut<'_, Self>, opacity: f32) -> PyRefMut<'_, Self> {
                slf.inner.style_mut().opacity = opacity;
                slf
            }

            #[doc = concat!(
                                                        "roughness(self, roughness: float) -> ",
                                                        stringify!($return_ty),
                                                        "\n\n",
                                                        "Set the surface roughness.\n"
                                                    )]
            pub fn roughness(mut slf: PyRefMut<'_, Self>, roughness: f32) -> PyRefMut<'_, Self> {
                slf.inner.style_mut().roughness = roughness;
                slf
            }

            #[doc = concat!(
                                                        "metallic(self, metallic: float) -> ",
                                                        stringify!($return_ty),
                                                        "\n\n",
                                                        "Set the surface metallic factor.\n"
                                                    )]
            pub fn metallic(mut slf: PyRefMut<'_, Self>, metallic: f32) -> PyRefMut<'_, Self> {
                slf.inner.style_mut().metallic = metallic;
                slf
            }
        }
    };
}
