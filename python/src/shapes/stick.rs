use super::style::impl_stylable;
use cosmol_viewer_core::shapes::Stick;
use pyo3::{PyRefMut, pyclass, pymethods};

#[pyclass(module = "cosmol_viewer", name = "Stick", from_py_object)]
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

impl_stylable!(PyStick);
