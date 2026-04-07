use crate::PyErr;
use crate::PyResult;
use crate::impl_stylable_pymethods;
use cosmol_viewer_core::shapes::{Molecule, Protein, Sphere, Stick};
use cosmol_viewer_core::utils::Stylable;
use cosmol_viewer_derive::gen_color_methods_submission;
use pyo3::{Bound, PyAny, PyRefMut, pyclass, pymethods};
use pyo3_stub_gen::derive::{gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::inventory::submit;

#[gen_stub_pyclass]
#[pyclass(name = "Sphere", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A sphere shape in the scene.

Parameters
----------
center : [float, float, float]
    The ``[x, y, z]`` coordinates of the sphere center.
radius : float
    The radius of the sphere.

Examples
--------
.. code-block:: python

    sphere = Sphere([0, 0, 0], 1.0).color("\#FF0000")
    sphere = Sphere([0, 0, 0], 1.0).color((255, 0, 0))
"#]
pub struct PySphere {
    pub inner: Sphere,
}

gen_color_methods_submission!(PySphere, Sphere);

#[gen_stub_pymethods]
#[pymethods]
impl PySphere {
    #[new]
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self {
            inner: Sphere::new(center, radius),
        }
    }

    #[doc = r#"
Set the radius of the sphere.

Parameters
----------
radius : float
    The new radius of the sphere.

Returns
-------
Sphere
    The updated sphere object.
"#]
    pub fn set_radius(mut slf: PyRefMut<'_, Self>, radius: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_radius(radius);
        slf
    }

    #[doc = r#"
Set the center of the sphere.

Parameters
----------
center : [float, float, float]
    The new center coordinates of the sphere.

Returns
-------
Sphere
    The updated sphere object.
"#]
    pub fn set_center(mut slf: PyRefMut<'_, Self>, center: [f32; 3]) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_center(center);
        slf
    }
}

impl_stylable_pymethods!(PySphere, Sphere);

#[gen_stub_pyclass]
#[pyclass(name = "Stick", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A cylindrical stick shape connecting two points.

Parameters
----------
start : [float, float, float]
    The starting point of the stick.
end : [float, float, float]
    The ending point of the stick.
thickness : float
    The radius or thickness of the stick.

Examples
--------
.. code-block:: python

    stick = Stick([0, 0, 0], [1, 1, 1], 0.1).opacity(0.5)
"#]
pub struct PyStick {
    pub inner: Stick,
}

gen_color_methods_submission!(PyStick, Stick);

#[gen_stub_pymethods]
#[pymethods]
impl PyStick {
    #[new]
    pub fn new(start: [f32; 3], end: [f32; 3], thickness: f32) -> Self {
        Self {
            inner: Stick::new(start, end, thickness),
        }
    }

    #[doc = r#"
Set the thickness of the stick.

Parameters
----------
thickness : float
    The new thickness of the stick.

Returns
-------
Stick
    The updated stick object.
"#]
    pub fn set_thickness(mut slf: PyRefMut<'_, Self>, thickness: f32) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_thickness(thickness);
        slf
    }

    #[doc = r#"
Set the starting point of the stick.

Parameters
----------
start : [float, float, float]
    The new starting point.

Returns
-------
Stick
    The updated stick object.
"#]
    pub fn set_start(mut slf: PyRefMut<'_, Self>, start: [f32; 3]) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_start(start);
        slf
    }

    #[doc = r#"
Set the ending point of the stick.

Parameters
----------
end : [float, float, float]
    The new ending point.

Returns
-------
Stick
    The updated stick object.
"#]
    pub fn set_end(mut slf: PyRefMut<'_, Self>, end: [f32; 3]) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.set_end(end);
        slf
    }
}

impl_stylable_pymethods!(PyStick, Stick);

#[gen_stub_pyclass]
#[pyclass(name = "Molecule", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A molecular shape object.

This class is typically created by parsing an SDF-format string.

Examples
--------
.. code-block:: python

    content = open("structure.sdf", "r").read()
    mol = Molecule.from_sdf(content).centered()
"#]
pub struct PyMolecule {
    pub inner: Molecule,
}

gen_color_methods_submission!(PyMolecule, Molecule);

#[gen_stub_pymethods]
#[pymethods]
impl PyMolecule {
    #[staticmethod]
    #[doc = r#"
Create a molecule from an SDF-format string.

Parameters
----------
sdf : str
    The SDF file content as a string.

Returns
-------
Molecule
    The parsed molecule object.
"#]
    pub fn from_sdf(sdf: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Molecule::from_sdf(sdf)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        })
    }

    #[doc = r#"
Get the geometric center of the molecule.

Returns
-------
[float, float, float]
    The center coordinates of the molecule.
"#]
    pub fn get_center(slf: PyRefMut<'_, Self>) -> [f32; 3] {
        slf.inner.clone().get_center()
    }

    #[doc = r#"
Center the molecule around the origin.

Returns
-------
Molecule
    The centered molecule object.
"#]
    pub fn centered(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().centered();
        slf
    }
}

impl_stylable_pymethods!(PyMolecule, Molecule);

#[gen_stub_pyclass]
#[pyclass(name = "Protein", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A protein shape object.

This class is typically created by parsing an mmCIF-format string.

Examples
--------
.. code-block:: python

    content = open("2AMD.cif", "r", encoding="utf-8").read()
    prot = Protein.from_mmcif(content).centered().color("\#F9FAFB")
"#]
pub struct PyProtein {
    pub inner: Protein,
}

gen_color_methods_submission!(PyProtein, Protein);

#[gen_stub_pymethods]
#[pymethods]
impl PyProtein {
    #[staticmethod]
    #[doc = r#"
Create a protein from an mmCIF-format string.

Parameters
----------
mmcif : str
    The mmCIF file content as a string.

Returns
-------
Protein
    The parsed protein object.
"#]
    pub fn from_mmcif(mmcif: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Protein::from_mmcif(mmcif)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        })
    }

    #[doc = r#"
Get the geometric center of the protein.

Returns
-------
[float, float, float]
    The center coordinates of the protein.
"#]
    pub fn get_center(slf: PyRefMut<'_, Self>) -> [f32; 3] {
        slf.inner.clone().get_center()
    }

    #[doc = r#"
Center the protein around the origin.

Returns
-------
Protein
    The centered protein object.
"#]
    pub fn centered(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().centered();
        slf
    }
}

impl_stylable_pymethods!(PyProtein, Protein);

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
