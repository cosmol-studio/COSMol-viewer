use crate::PyErr;
use crate::PyResult;
use crate::impl_stylable_pymethods;
use cosmol_viewer_core::shapes::{Molecule, Protein, Sphere, Stick};
use cosmol_viewer_core::utils::Stylable;
#[cfg(feature = "stubgen")]
use cosmol_viewer_derive::gen_color_methods_submission;
use pyo3::{Bound, PyAny, PyRefMut, pyclass, pymethods};
#[cfg(feature = "stubgen")]
use pyo3_stub_gen::derive::{gen_methods_from_python, gen_stub_pyclass, gen_stub_pymethods};
#[cfg(feature = "stubgen")]
use pyo3_stub_gen::inventory::submit;
#[cfg(not(feature = "stubgen"))]
use pyo3_stub_gen_derive::remove_gen_stub;

#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
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

#[cfg(feature = "stubgen")]
gen_color_methods_submission!(PySphere, Sphere);

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
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

#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
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

#[cfg(feature = "stubgen")]
gen_color_methods_submission!(PyStick, Stick);

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
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

#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
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

#[cfg(feature = "stubgen")]
gen_color_methods_submission!(PyMolecule, Molecule);

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
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
    pub fn from_cosmolkit(molecule: &Bound<'_, PyAny>) -> PyResult<Self> {
        match cosmolkit_molecule_to_viewer_molecule(molecule) {
            Ok(inner) => Ok(Self { inner }),
            Err(direct_error) => {
                let sdf = cosmolkit_molecule_to_sdf(molecule).map_err(|sdf_error| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                        "Failed to convert cosmolkit molecule directly: {direct_error}; \
                         SDF fallback also failed: {sdf_error}"
                    ))
                })?;
                Self::from_sdf(&sdf)
            }
        }
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
        color: Option<Bound<'_, PyAny>>,
        width: Option<f32>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        let color = match color {
            Some(color) => py_to_color(color)?,
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
        color: Option<Bound<'_, PyAny>>,
        width: Option<f32>,
    ) -> PyResult<PyRefMut<'a, Self>> {
        let width = width.unwrap_or(slf.inner.outline.width);
        slf.inner = match color {
            Some(color) => {
                let color = py_to_color(color)?;
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

fn cosmolkit_molecule_to_viewer_molecule(molecule: &Bound<'_, PyAny>) -> PyResult<Molecule> {
    let molecule = molecule
        .call_method1("with_kekulized_bonds", (true,))
        .unwrap_or_else(|_| molecule.clone());

    if let Ok(molecule) = try_cosmolkit_molecule_to_viewer_molecule(&molecule) {
        return Ok(molecule);
    }

    let molecule = molecule.call_method0("with_2d_coords")?;
    try_cosmolkit_molecule_to_viewer_molecule(&molecule)
}

fn try_cosmolkit_molecule_to_viewer_molecule(molecule: &Bound<'_, PyAny>) -> PyResult<Molecule> {
    let atom_atomic_numbers = cosmolkit_atom_atomic_numbers(molecule)?;
    let atom_posits = cosmolkit_coords(molecule, atom_atomic_numbers.len())?;
    let (bond_order_codes, bond_indices) = cosmolkit_bonds(molecule)?;

    Molecule::from_atom_bond_data(
        atom_atomic_numbers,
        atom_posits,
        bond_order_codes,
        bond_indices,
        None,
    )
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

fn cosmolkit_coords(molecule: &Bound<'_, PyAny>, atom_count: usize) -> PyResult<Vec<[f32; 3]>> {
    if let Ok(coords) = molecule.call_method0("coords_3d") {
        return py_rows_to_vec3(&coords, atom_count);
    }

    let coords = molecule.call_method0("coords_2d")?;
    py_rows_to_vec3(&coords, atom_count)
}

fn py_rows_to_vec3(coords: &Bound<'_, PyAny>, atom_count: usize) -> PyResult<Vec<[f32; 3]>> {
    let rows: Vec<Vec<f64>> = coords.call_method0("tolist")?.extract()?;
    if rows.len() < atom_count {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "COSMolKit exposed {} coordinates for {atom_count} atoms",
            rows.len()
        )));
    }
    if rows.len() > atom_count {
        eprintln!(
            "[WARN] COSMolKit exposed {} coordinates for {atom_count} atoms; ignoring trailing coordinates",
            rows.len()
        );
    }
    rows.into_iter()
        .take(atom_count)
        .enumerate()
        .map(|(index, row)| {
            if row.len() < 2 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "coordinate row {index} has {} columns, expected at least 2",
                    row.len()
                )));
            }
            Ok([
                row[0] as f32,
                row[1] as f32,
                row.get(2).copied().unwrap_or(0.0) as f32,
            ])
        })
        .collect()
}

fn cosmolkit_atom_atomic_numbers(molecule: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    let atoms = molecule.call_method0("atoms")?;
    let mut atomic_numbers = Vec::new();
    for atom in atoms.try_iter()? {
        let atom = atom?;
        let atomic_num: usize = atom.call_method0("atomic_num")?.extract()?;
        atomic_numbers.push(u8::try_from(atomic_num).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "COSMolKit atom atomic number {atomic_num} is out of u8 range"
            ))
        })?);
    }
    Ok(atomic_numbers)
}

fn cosmolkit_bonds(molecule: &Bound<'_, PyAny>) -> PyResult<(Vec<i64>, Vec<[usize; 2]>)> {
    let bonds = molecule.call_method0("bonds")?;
    let mut bond_order_codes = Vec::new();
    let mut bond_indices = Vec::new();
    for bond in bonds.try_iter()? {
        let bond = bond?;
        let begin_atom_idx = bond.call_method0("begin_atom_idx")?.extract()?;
        let end_atom_idx = bond.call_method0("end_atom_idx")?.extract()?;
        let bond_type_code = bond.call_method0("bond_type_code")?.extract()?;
        bond_indices.push([begin_atom_idx, end_atom_idx]);
        bond_order_codes.push(bond_type_code);
    }
    Ok((bond_order_codes, bond_indices))
}

fn cosmolkit_molecule_to_sdf(molecule: &Bound<'_, PyAny>) -> PyResult<String> {
    let molecule = molecule
        .call_method1("with_kekulized_bonds", (true,))
        .unwrap_or_else(|_| molecule.clone());

    match molecule.call_method1("to_sdf_string", ("v3000",)) {
        Ok(sdf) => sdf.extract(),
        Err(first_error) => {
            if let Ok(with_coords) = molecule.call_method0("with_2d_coords") {
                if let Ok(sdf) = with_coords.call_method1("to_sdf_string", ("v3000",)) {
                    return sdf.extract();
                }
            }

            if let Ok(sdf) = molecule.call_method1("to_sdf_string", ("v2000",)) {
                return sdf.extract();
            }

            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Expected a cosmolkit.Molecule-like object with to_sdf_string(format). \
                 Direct export failed: {first_error}"
            )))
        }
    }
}

impl_stylable_pymethods!(PyMolecule, Molecule);

#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
#[pyclass(name = "Protein", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A protein shape object.

This class is typically created by parsing an mmCIF- or PDB-format string.
Protein rendering uses the Rust core cartoon pipeline: secondary structure is
assigned from backbone geometry, and the displayed ribbon mesh is generated with
the ChimeraX-style spline, cross-section, and cap/extrusion path.

Examples
--------
.. code-block:: python

    content = open("2AMD.cif", "r", encoding="utf-8").read()
    prot = Protein.from_mmcif(content).centered().rainbow_residues()

Use ``.color("\#F9FAFB")`` for a uniform cartoon color.
"#]
pub struct PyProtein {
    pub inner: Protein,
}

#[cfg(feature = "stubgen")]
gen_color_methods_submission!(PyProtein, Protein);

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
#[pymethods]
impl PyProtein {
    #[staticmethod]
    #[doc = r#"
Create a protein from an mmCIF-format string.

The parser reads backbone atoms needed for cartoon rendering. Secondary
structure is assigned by the Rust core before mesh generation.

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

    #[staticmethod]
    #[doc = r#"
Create a protein from a PDB-format string.

The parser reads backbone atoms needed for cartoon rendering. Secondary
structure is assigned by the Rust core before mesh generation.

Parameters
----------
pdb : str
    The PDB file content as a string.

Returns
-------
Protein
    The parsed protein object.
"#]
    pub fn from_pdb(pdb: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Protein::from_pdb(pdb)
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

    #[doc = r#"
Color the protein cartoon with ChimeraX-style rainbow coloring by residue.

Each biopolymer chain is colored independently from blue at the first rendered
residue to red at the last rendered residue, matching ChimeraX ``rainbow`` /
``color sequential residues`` default behavior for cartoon ribbons.

Returns
-------
Protein
    The updated protein object.
"#]
    pub fn rainbow_residues(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().rainbow_residues();
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

    if let Ok(s) = color.extract::<String>() {
        return Color::try_from(s.as_str())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()));
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Color must be [int;3] with each value in [0, 255], or hex string like '#ffffff'",
    ))
}
