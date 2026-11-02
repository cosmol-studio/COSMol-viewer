//! Foreign CK objects remain the selection owners; captured data is read-only.
use crate::external::{External, ExternalType};
use cosmol_viewer_core::display::input::*;
use cosmolkit as ck;
use pyo3::{exceptions::PyValueError, prelude::*};
use std::sync::Arc;

pub struct MoleculeType;
pub struct ProteinType;
pub struct ElementType;
macro_rules! foreign_type {
    ($ty:ident,$name:literal) => {
        impl ExternalType for $ty {
            const MODULE: &'static str = "cosmolkit";
            const NAME: &'static str = $name;
        }
    };
}
foreign_type!(MoleculeType, "Molecule");
foreign_type!(ProteinType, "Protein");
foreign_type!(ElementType, "Element");
#[derive(FromPyObject)]
pub(crate) enum ScientificInput<'py> {
    Molecule(External<'py, MoleculeType>),
    Protein(External<'py, ProteinType>),
}
impl ScientificInput<'_> {
    pub(crate) fn object(&self) -> &Bound<'_, PyAny> {
        match self {
            Self::Molecule(v) => v.as_bound(),
            Self::Protein(v) => v.as_bound(),
        }
    }
    pub(crate) fn capture(&self) -> PyResult<SourceInput> {
        match self {
            Self::Molecule(v) => molecule(v.as_bound()).map(SourceInput::Molecule),
            Self::Protein(v) => protein(v.as_bound()).map(SourceInput::Protein),
        }
    }
}
fn value_error(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}
fn read<'py>(object: &Bound<'py, PyAny>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    let value = object.getattr(name)?;
    if value.is_callable() {
        value.call0()
    } else {
        Ok(value)
    }
}
fn rows<const N: usize>(object: &Bound<'_, PyAny>) -> PyResult<Vec<[f64; N]>> {
    let rows: Vec<Vec<f64>> = object.call_method0("tolist")?.extract()?;
    rows.into_iter()
        .map(|row| {
            if row.iter().any(|&v| !(v as f32).is_finite()) {
                return Err(value_error("coordinates must fit finite f32 values"));
            }
            row.try_into()
                .map_err(|_| value_error("incorrect coordinate dimensions"))
        })
        .collect()
}
fn molecule(source: &Bound<'_, PyAny>) -> PyResult<MoleculeInput> {
    let elements = read(source, "atoms")?
        .try_iter()?
        .map(|a| {
            let n = read(&a?, "atomic_number")?.extract::<u8>()?;
            ck::Element::from_atomic_number(n).ok_or_else(|| value_error("invalid atomic number"))
        })
        .collect::<PyResult<Vec<_>>>()?;
    let bonds = read(source, "bonds")?
        .try_iter()?
        .map(|b| {
            let b = b?;
            let code = read(&b, "order_code")?.extract()?;
            Ok(BondInput {
                begin: read(&b, "begin")?.extract()?,
                end: read(&b, "end")?.extract()?,
                order: ck::BondOrder::from_rdkit_code(code)
                    .ok_or_else(|| value_error("invalid bond order"))?,
                aromatic: read(&b, "is_aromatic")?.extract()?,
            })
        })
        .collect::<PyResult<Vec<_>>>()?;
    let conformers = read(source, "conformers_3d")?
        .try_iter()?
        .map(|c| {
            let c = c?;
            Ok(ConformerInput {
                id: read(&c, "id")?.extract()?,
                coordinates: rows(&read(&c, "coordinates")?)?,
            })
        })
        .collect::<PyResult<Vec<_>>>()?;
    let xy = read(source, "coordinates_2d")?;
    let coordinates_2d = if xy.is_none() { None } else { Some(rows(&xy)?) };
    let atom_weights = (0..elements.len())
        .map(|i| {
            source
                .call_method1("atom_property_double", (i, "WEIGHT"))?
                .extract()
        })
        .collect::<PyResult<Vec<_>>>()?;
    let fields: Vec<(String, String)> =
        read(&read(source, "properties")?, "sdf_data_fields")?.extract()?;
    let field = |key: &str| {
        fields
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
    };
    Ok(MoleculeInput {
        elements,
        bonds,
        conformers,
        coordinates_2d,
        atom_weights,
        dummy_weights: field("DUMMY_WEIGHTS"),
        dummy_indices: field("DUMMY_ATOM_INDICES"),
    })
}
fn protein(source: &Bound<'_, PyAny>) -> PyResult<ProteinInput> {
    let mut chains = Vec::new();
    for chain in read(source, "chains")?.try_iter()? {
        let mut residues = Vec::new();
        for residue in read(&chain?, "residues")?.try_iter()? {
            let residue = residue?;
            let name: String = read(&residue, "name")?.extract()?;
            let row = read(&residue, "row")?;
            let src = read(&row, "source")?;
            let seq = read(&src, "seq_id")?;
            let seq_id = if seq.is_none() {
                None
            } else {
                Some((
                    read(&seq, "seq_num")?.extract()?,
                    read(&seq, "ins_code")?.extract()?,
                ))
            };
            let mut atoms = Vec::new();
            for atom in read(&residue, "atoms")?.try_iter()? {
                let atom = atom?;
                let row = read(&atom, "row")?;
                let p: [f64; 3] = read(&atom, "position")?.extract()?;
                if p.iter().any(|&v| !(v as f32).is_finite()) {
                    return Err(value_error("coordinates must fit finite f32 values"));
                }
                let altloc = read(&row, "altloc")?;
                atoms.push(ProteinAtomInput {
                    id: ck::BioAtomId::new(read(&atom, "id")?.extract()?),
                    name: read(&read(&atom, "name")?, "as_str")?
                        .extract::<String>()?
                        .trim()
                        .to_owned(),
                    position: p.map(|v| v as f32).into(),
                    source_position: p,
                    atomic_number: read(&read(&atom, "element")?, "atomic_number")?.extract()?,
                    alt_loc: if altloc.is_none() {
                        None
                    } else {
                        Some(read(&altloc, "value")?.extract()?)
                    },
                    occupancy: read(&row, "occupancy")?.extract()?,
                    b_factor: read(&row, "b_iso")?.extract()?,
                });
            }
            residues.push(ProteinResidueInput {
                id: ck::BioResidueId::new(read(&residue, "id")?.extract()?),
                residue_type: ck::find_residue_info(&name).code,
                name,
                atoms,
                seq_id,
                label_seq_id: read(&src, "label_seq_id")?.extract()?,
            });
        }
        chains.push(residues);
    }
    let mmcif = source
        .py()
        .import("cosmolkit")?
        .getattr("BioCoordinateFormat")?
        .getattr("Mmcif")?;
    let format = read(source, "input_format")?;
    let pdb = source
        .py()
        .import("cosmolkit")?
        .getattr("BioCoordinateFormat")?
        .getattr("Pdb")?;
    let input_format = if format.eq(&mmcif)? {
        ck::BioCoordinateFormat::Mmcif
    } else if format.eq(&pdb)? {
        ck::BioCoordinateFormat::Pdb
    } else {
        return Err(value_error("expected a PDB or mmCIF protein source"));
    };
    Ok(ProteinInput {
        chains,
        input_format,
    })
}

struct Snapshot {
    owner: Py<PyAny>,
    input: SourceInput,
}
#[derive(Clone)]
#[pyclass(module = "cosmol_viewer", from_py_object)]
pub struct Selector {
    snapshot: Arc<Snapshot>,
}
#[derive(Clone)]
#[pyclass(module = "cosmol_viewer", from_py_object)]
pub struct Selection {
    snapshot: Arc<Snapshot>,
    kind: EntityKind,
    ids: Vec<usize>,
}
/// Select entities from an actual CK object without changing its data.
#[pyfunction]
pub(crate) fn select(source: ScientificInput<'_>) -> PyResult<Selector> {
    let input = source.capture()?;
    Ok(Selector {
        snapshot: Arc::new(Snapshot {
            owner: source.object().clone().unbind(),
            input,
        }),
    })
}
impl Selector {
    fn entities(&self, kind: EntityKind) -> PyResult<Selection> {
        Ok(Selection {
            snapshot: self.snapshot.clone(),
            kind,
            ids: self.snapshot.input.ids(kind).map_err(value_error)?,
        })
    }
}
#[pymethods]
impl Selector {
    pub fn atoms(&self) -> PyResult<Selection> {
        self.entities(EntityKind::Atoms)
    }
    pub fn bonds(&self) -> PyResult<Selection> {
        self.entities(EntityKind::Bonds)
    }
    pub fn residues(&self) -> PyResult<Selection> {
        self.entities(EntityKind::Residues)
    }
}
#[pymethods]
impl Selection {
    /// Return a filtered selection; the original selection is reusable.
    pub(crate) fn element(&self, element: External<'_, ElementType>) -> PyResult<Self> {
        let n = read(element.as_bound(), "atomic_number")?.extract()?;
        let e = ck::Element::from_atomic_number(n).ok_or_else(|| value_error("invalid element"))?;
        let mut output = self.clone();
        output
            .snapshot
            .input
            .filter_element(output.kind, &mut output.ids, e)
            .map_err(value_error)?;
        Ok(output)
    }
    pub fn aromatic(&self) -> PyResult<Self> {
        let mut output = self.clone();
        output
            .snapshot
            .input
            .filter_aromatic(output.kind, &mut output.ids)
            .map_err(value_error)?;
        Ok(output)
    }
    #[getter]
    pub fn ids(&self) -> Vec<usize> {
        self.ids.clone()
    }
    fn __len__(&self) -> usize {
        self.ids.len()
    }
}
impl Selection {
    pub(crate) fn project(
        &self,
        source: ScientificInput<'_>,
        style: cosmol_viewer_core::styles::Representation,
    ) -> PyResult<cosmol_viewer_core::display::Display> {
        if !source
            .object()
            .is(self.snapshot.owner.bind(source.object().py()))
        {
            return Err(value_error(
                "selection belongs to a different source object",
            ));
        }
        if source.capture()? != self.snapshot.input {
            return Err(value_error(
                "source rendering data changed since selection; create a new selection",
            ));
        }
        self.snapshot
            .input
            .project(self.kind, &self.ids, style)
            .map_err(value_error)
    }
}
