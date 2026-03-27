use crate::PyErr;
use crate::PyResult;
use cosmol_viewer_core::{
    shapes::{Molecule, Protein, Sphere, Stick},
    utils::VisualShape,
};
use pyo3::{Bound, PyAny, PyRefMut, pyclass, pymethods};
use pyo3_stub_gen::derive::{gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::inventory::submit;

#[gen_stub_pyclass]
#[pyclass(name = "Sphere", from_py_object)]
#[derive(Clone)]
#[doc = r#"
    A sphere shape in the scene.

    # Args
    - center: [x, y, z] coordinates of the sphere center.
    - radius: Radius of the sphere.

    # Example
    ```python
    sphere = Sphere([0, 0, 0], 1.0).color([1.0, 0.0, 0.0])
    ```
"#]
pub struct PySphere {
    pub inner: Sphere,
}

submit! {
    gen_methods_from_python! {
        r#"
        class PySphere:
            from typing import overload, Self

            @overload
            def color(self, c: tuple[int, int, int]) -> Sphere: ...

            @overload
            def color(self, c: str) -> Sphere: ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PySphere {
    #[new]
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self {
            inner: Sphere::new(center, radius),
        }
    }

    pub fn set_radius(mut slf: PyRefMut<'_, Self>, radius: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_radius(radius);
        slf
    }

    pub fn set_center(mut slf: PyRefMut<'_, Self>, center: [f32; 3]) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_center(center);
        slf
    }

    #[gen_stub(skip)]
    pub fn color<'py>(
        mut slf: PyRefMut<'py, Self>,
        color: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let color = py_to_color(color)?;
        slf.inner = slf.inner.color(color);
        Ok(slf)
    }

    pub fn opacity(mut slf: PyRefMut<'_, Self>, opacity: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.opacity(opacity);
        slf
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "Stick", from_py_object)]
#[derive(Clone)]
#[doc = r#"
    A cylindrical stick (or capsule) connecting two points.

    # Args
    - start: Starting point [x, y, z].
    - end: Ending point [x, y, z].
    - thickness: Stick radius.

    # Example
    ```python
    stick = Stick([0,0,0], [1,1,1], 0.1).opacity(0.5)
    ```
"#]
pub struct PyStick {
    pub inner: Stick,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyStick {
    #[new]
    pub fn new(start: [f32; 3], end: [f32; 3], thickness: f32) -> Self {
        Self {
            inner: Stick::new(start, end, thickness),
        }
    }

    #[gen_stub(skip)]
    pub fn color<'py>(
        mut slf: PyRefMut<'py, Self>,
        color: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let color = py_to_color(color)?;
        slf.inner = slf.inner.color(color);
        Ok(slf)
    }

    pub fn set_thickness(mut slf: PyRefMut<'_, Self>, thickness: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_thickness(thickness);
        slf
    }

    pub fn set_start(mut slf: PyRefMut<'_, Self>, start: [f32; 3]) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_start(start);
        slf
    }

    pub fn set_end(mut slf: PyRefMut<'_, Self>, end: [f32; 3]) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_end(end);
        slf
    }

    pub fn opacity(mut slf: PyRefMut<'_, Self>, opacity: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.opacity(opacity);
        slf
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "Molecule", from_py_object)]
#[derive(Clone)]
#[doc = r#"
    A molecular shape object.
    Typically created by parsing an SDF format string.

    # Example
    ```python
    # Load from file content
    content = open("structure.sdf", "r").read()
    mol = Molecule.from_sdf(content).centered().color([0, 1, 0])
    ```
"#]
pub struct PyMolecule {
    pub inner: Molecule,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMolecule {
    #[staticmethod]
    #[doc = r#"
        Create a Molecule from an SDF format string.

        # Args
        - sdf: The SDF file content as a string.

        # Returns
        - Molecule: The parsed molecule object.
    "#]
    pub fn from_sdf(sdf: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Molecule::from_sdf(sdf)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        })
    }

    pub fn get_center(slf: PyRefMut<'_, Self>) -> [f32; 3] {
        slf.inner.clone().get_center()
    }

    pub fn centered(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().centered();
        slf
    }

    #[gen_stub(skip)]
    pub fn color<'py>(
        mut slf: PyRefMut<'py, Self>,
        color: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let color = py_to_color(color)?;
        slf.inner = slf.inner.clone().color(color);
        Ok(slf)
    }

    pub fn opacity(mut slf: PyRefMut<'_, Self>, opacity: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().opacity(opacity);
        slf
    }
}

#[gen_stub_pyclass]
#[pyclass(name = "Protein", from_py_object)]
#[derive(Clone)]
#[doc = r#"
    A protein shape object.
    Typically created by parsing an mmCIF format string.

    # Example
    ```python
    # Load from file content
    content = open("2AMD.cif", "r").read()
    prot = Protein.from_mmcif(content).centered().color([0, 1, 0])
    ```
"#]
pub struct PyProtein {
    pub inner: Protein,
}

submit! {
    gen_methods_from_python! {
        r#"
        class PyProtein:
            from typing import overload, Self

            @overload
            def color(self, c: tuple[int, int, int]) -> Protein: ...

            @overload
            def color(self, c: str) -> Protein: ...
        "#
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyProtein {
    #[staticmethod]
    #[doc = r#"
        Create a Protein from an mmCIF format string.

        # Args
        - mmcif: The mmCIF file content as a string.

        # Returns
        - Protein: The parsed protein object.
    "#]
    pub fn from_mmcif(mmcif: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Protein::from_mmcif(mmcif)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        })
    }

    pub fn get_center(slf: PyRefMut<'_, Self>) -> [f32; 3] {
        slf.inner.clone().get_center()
    }

    pub fn centered(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().centered();
        slf
    }

    #[gen_stub(skip)]
    pub fn color<'py>(
        mut slf: PyRefMut<'py, Self>,
        color: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let color = py_to_color(color)?;
        slf.inner = slf.inner.clone().color(color);
        Ok(slf)
    }

    pub fn opacity(mut slf: PyRefMut<'_, Self>, opacity: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().opacity(opacity);
        slf
    }
}

use cosmol_viewer_core::utils::Color;
use pyo3::types::PyAnyMethods;
pub fn py_to_color(color: Bound<'_, pyo3::PyAny>) -> PyResult<Color> {
    if let Ok(v) = color.extract::<[i64; 3]>() {
        for &c in &v {
            if c < 0 || c > 255 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Color value {} out of range [0, 255]",
                    c
                )));
            }
        }

        let v_u8 = [v[0] as u8, v[1] as u8, v[2] as u8];
        return Ok(Color::from(v_u8));
    }

    if let Ok(s) = color.extract::<&str>() {
        return Color::try_from(s)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Color must be [int;3] with each value in [0, 255], or hex string like '#ffffff'",
    ))
}
