use super::style::impl_stylable;
use cosmol_viewer_core::shapes::Protein;
use pyo3::{PyErr, PyResult};
use pyo3::{PyRefMut, pyclass, pymethods};

#[pyclass(module = "cosmol_viewer", name = "Protein", from_py_object)]
#[derive(Clone)]
#[doc = r#"
A protein shape object.

This class is typically created from an mmCIF- or PDB-format string parsed by
COSMolKit's protein reader. The viewer core assigns secondary structure from
backbone geometry and generates the displayed ChimeraX-style cartoon ribbon.

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

#[pymethods]
impl PyProtein {
    #[staticmethod]
    #[doc = r#"
Create a protein from an mmCIF-format string.

Parsing is delegated to COSMolKit's protein reader. The viewer core assigns
secondary structure from the resulting backbone geometry before mesh generation.

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

Parsing is delegated to COSMolKit's protein reader. The viewer core assigns
secondary structure from the resulting backbone geometry before mesh generation.

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

    #[doc = r#"
Display the protein as a ChimeraX-compatible solvent-excluded surface.

The default probe radius is 1.4 angstrom and the grid spacing is 0.5 angstrom.
The generated surface participates in scene opacity and depth cueing.

Returns
-------
Protein
    The updated protein object.
"#]
    pub fn surface(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().surface();
        slf
    }

    #[doc = r#"
Display the protein as a solvent-accessible surface.

Returns
-------
Protein
    The updated protein object.
"#]
    pub fn solvent_accessible_surface(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().solvent_accessible_surface();
        slf
    }

    #[pyo3(signature = (probe_radius=1.4, grid_spacing=0.5, solvent_accessible=false, sharp_boundaries=true))]
    #[doc = r#"
Display the protein as a molecular surface with explicit parameters.

Parameters
----------
probe_radius : float
    Solvent probe radius in angstrom.
grid_spacing : float
    Distance-grid spacing in angstrom.
solvent_accessible : bool
    Return the solvent-accessible surface instead of the solvent-excluded surface.
sharp_boundaries : bool
    Subdivide atom patches so their shared edges are exact.

Returns
-------
Protein
    The updated protein object.
"#]
    pub fn surface_with_options(
        mut slf: PyRefMut<'_, Self>,
        probe_radius: f32,
        grid_spacing: f32,
        solvent_accessible: bool,
        sharp_boundaries: bool,
    ) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().surface_with_settings(
            cosmol_viewer_core::shapes::ProteinSurfaceOptions {
                probe_radius,
                grid_spacing,
                solvent_accessible,
                sharp_boundaries,
            },
        );
        slf
    }

    #[doc = r#"
Display the protein as a cartoon ribbon.

Returns
-------
Protein
    The updated protein object.
"#]
    pub fn ribbon(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.inner = slf.inner.clone().ribbon();
        slf
    }
}

impl_stylable!(PyProtein);
