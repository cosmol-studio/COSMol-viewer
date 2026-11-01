use super::style::impl_stylable;
use cosmol_viewer_core::shapes::Molecule;
use cosmol_viewer_core::utils::Color;
use pyo3::{PyErr, PyResult};
use pyo3::{PyRefMut, pyclass, pymethods};

use super::cosmolkit::{MoleculeInput, into_viewer_molecule};
use crate::input::ColorInput;

#[pyclass(module = "cosmol_viewer", name = "Molecule", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A molecular shape object.

This class is typically created from an SDF-format string parsed by COSMolKit.

Examples
--------
.. code-block:: python

    content = open("structure.sdf", "r").read()
    mol = Molecule.from_sdf(content).centered()
"#]
pub struct PyMolecule {
    pub inner: Molecule,
}

#[pymethods]
impl PyMolecule {
    #[staticmethod]
    #[doc = r#"
Create a molecule from an SDF-format string.

SDF parsing is delegated to COSMolKit; the viewer receives atoms, bonds, and
coordinates from the parsed molecule.

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

    #[staticmethod]
    #[doc = r#"
Create a viewer molecule from a Python ``cosmolkit.Molecule`` object.

The conversion is performed on the Rust side by exporting the COSMolKit
molecule atoms, bonds, and coordinates into the viewer molecule format.
3D conformer coordinates are preferred over 2D coordinates. If the COSMolKit
molecule has no stored coordinates, 2D coordinates are computed first.

Parameters
----------
molecule : cosmolkit.Molecule
    A molecule object from the Python ``cosmolkit`` package built against a
    compatible COSMolKit version.

Returns
-------
Molecule
    The converted viewer molecule object.
"#]
    fn from_cosmolkit(molecule: MoleculeInput<'_>) -> PyResult<Self> {
        Ok(Self {
            inner: into_viewer_molecule(molecule)?,
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

    #[doc = r#"
Use the default ball-and-stick molecule style.

Atoms are rendered as spheres and bonds are rendered as sticks.

Returns
-------
Molecule
    The updated molecule object.
"#]
    pub fn ball_and_stick(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().ball_and_stick();
        slf
    }

    #[doc = r#"
Use a stick-only molecule style.

Bonds are rendered as sticks with same-radius endpoint spheres for smooth joins,
following the ChimeraX stick-style model. Double and triple bonds remain visible
as thinner parallel sticks, with spacing derived from the ChimeraX stick radius.
Aromatic ring bonds render as a single stick plus an inner aromatic line. Van der
Waals atom spheres are not rendered, so the result remains a compact stick
depiction where atom colors are carried by the bond segments and junction caps.

Returns
-------
Molecule
    The updated molecule object.
"#]
    pub fn stick(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().stick();
        slf
    }

    #[doc = r#"
Use a sphere-only molecule style.

Atoms are rendered as spheres and bonds are not rendered.

Returns
-------
Molecule
    The updated molecule object.
"#]
    pub fn sphere(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().sphere();
        slf
    }

    #[doc = r#"
Configure the molecule outline.

Parameters
----------
enabled : bool
    Whether to render an outline around this molecule.
color : tuple[int, int, int] or str, optional
    The outline color as an RGB tuple or a hex string. If omitted, the current
    outline color is kept.
width : float, optional
    The outline width in scene units. If omitted, the current outline width is kept.

Returns
-------
Molecule
    The updated molecule object.
"#]
    #[pyo3(signature = (enabled, color=None, width=None))]
    pub fn set_outline<'a>(
        mut slf: PyRefMut<'a, Self>,
        enabled: bool,
        color: Option<ColorInput>,
        width: Option<f32>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        let color = match color {
            Some(color) => color.into_color()?,
            None => Color::from(slf.inner.outline.color),
        };
        let width = width.unwrap_or(slf.inner.outline.width);
        slf.inner = slf.inner.clone().set_outline(enabled, color, width);
        Ok(slf)
    }

    #[doc = r#"
Enable an outline around this molecule.

Parameters
----------
color : tuple[int, int, int] or str, optional
    The outline color as an RGB tuple or a hex string. If omitted, the current
    outline color is kept.
width : float, optional
    The outline width in scene units. If omitted, the current outline width is kept.

Returns
-------
Molecule
    The updated molecule object.
"#]
    #[pyo3(signature = (color=None, width=None))]
    pub fn enable_outline<'a>(
        mut slf: PyRefMut<'a, Self>,
        color: Option<ColorInput>,
        width: Option<f32>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        let width = width.unwrap_or(slf.inner.outline.width);
        slf.inner = match color {
            Some(color) => {
                let color = color.into_color()?;
                slf.inner.clone().set_outline(true, color, width)
            }
            None => slf.inner.clone().enable_outline(width),
        };
        Ok(slf)
    }

    #[doc = r#"
Disable the outline around this molecule.

Returns
-------
Molecule
    The updated molecule object.
"#]
    pub fn disable_outline(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().disable_outline();
        slf
    }
}

impl_stylable!(PyMolecule);
