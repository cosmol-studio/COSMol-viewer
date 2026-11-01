use super::style::impl_stylable;
use cosmol_viewer_core::shapes::Sphere;
use pyo3::{PyRefMut, pyclass, pymethods};

#[pyclass(module = "cosmol_viewer", name = "Sphere", from_py_object)]
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

impl_stylable!(PySphere);
