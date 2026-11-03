//! Python owns the foreign handle; all selection and scene behavior is native.
use crate::external::{External, ExternalType};
use cosmol_viewer_core::selection as native;
use cosmolkit as ck;
use pyo3::{exceptions::PyValueError, prelude::*};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock, Weak},
};

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

fn value_error(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

#[derive(FromPyObject)]
pub(crate) enum ScientificInput<'py> {
    Molecule(External<'py, MoleculeType>),
    Protein(External<'py, ProteinType>),
}

pub(crate) enum NativeSource {
    Molecule(ck::Molecule),
    Protein(ck::Protein),
}
pub(crate) struct ImportedSource {
    owner: Py<PyAny>,
    pub(crate) native: NativeSource,
}
// Weak entries never extend a source's lifetime. Selectors, selections and Scenes
// retain imports, including the Python owner so its address cannot be reused.
static IMPORTS: OnceLock<Mutex<HashMap<usize, Weak<ImportedSource>>>> = OnceLock::new();
#[derive(Clone, Default)]
pub(crate) struct Sources(HashMap<usize, Arc<ImportedSource>>);
impl Sources {
    pub(crate) fn retain(&mut self, source: Arc<ImportedSource>) {
        self.0.insert(source.owner.as_ptr() as usize, source);
    }
}
impl ScientificInput<'_> {
    fn object(&self) -> &Bound<'_, PyAny> {
        match self {
            Self::Molecule(v) => v.as_bound(),
            Self::Protein(v) => v.as_bound(),
        }
    }
    pub(crate) fn import(&self) -> PyResult<Arc<ImportedSource>> {
        let key = self.object().as_ptr() as usize;
        let imports = IMPORTS.get_or_init(Default::default);
        if let Some(source) = imports.lock().unwrap().get(&key).and_then(Weak::upgrade) {
            return Ok(source);
        }
        // Do not hold the cache lock while calling foreign Python code.
        let native = match self {
            Self::Molecule(value) => {
                let archive = value.as_bound().call_method0("to_transfer_binary")?;
                let archive = archive.cast::<pyo3::types::PyBytes>()?;
                NativeSource::Molecule(
                    ck::Molecule::from_transfer_binary(archive.as_bytes()).map_err(value_error)?,
                )
            }
            Self::Protein(value) => {
                let archive = value.as_bound().call_method0("to_transfer_binary")?;
                let archive = archive.cast::<pyo3::types::PyBytes>()?;
                NativeSource::Protein(
                    ck::Protein::from_transfer_binary(archive.as_bytes()).map_err(value_error)?,
                )
            }
        };
        let source = Arc::new(ImportedSource {
            owner: self.object().clone().unbind(),
            native,
        });
        let mut cache = imports.lock().unwrap();
        cache.retain(|_, value| value.strong_count() > 0);
        // A foreign call may release the GIL; share any concurrent successful import.
        Ok(cache.entry(key).or_default().upgrade().unwrap_or_else(|| {
            cache.insert(key, Arc::downgrade(&source));
            source
        }))
    }
}

#[derive(Clone)]
#[pyclass(module = "cosmol_viewer", from_py_object)]
pub struct Selector {
    source: Arc<ImportedSource>,
}
#[derive(Clone)]
enum Ids {
    MoleculeAtoms(Vec<ck::AtomId>),
    MoleculeBonds(Vec<ck::BondId>),
    ProteinAtoms(Vec<ck::BioAtomId>),
    ProteinResidues(Vec<ck::BioResidueId>),
}
pub(crate) enum NativeSelection<'a> {
    MoleculeAtoms(native::MoleculeAtomSelection<'a>),
    MoleculeBonds(native::MoleculeBondSelection<'a>),
    ProteinAtoms(native::ProteinAtomSelection<'a>),
    ProteinResidues(native::ProteinResidueSelection<'a>),
}
#[derive(Clone)]
#[pyclass(module = "cosmol_viewer", from_py_object)]
pub struct Selection {
    pub(crate) source: Arc<ImportedSource>,
    ids: Ids,
}
/// Import a CK object once and select directly from its native Rust snapshot.
/// Live selectors, selections and Scenes share the import. To import edited data
/// while an old snapshot is still in use, pass a new CK object (for example a copy).
#[pyfunction]
pub(crate) fn select(source: ScientificInput<'_>) -> PyResult<Selector> {
    Ok(Selector {
        source: source.import()?,
    })
}
#[pymethods]
impl Selector {
    pub fn atoms(&self) -> PyResult<Selection> {
        let ids = match &self.source.native {
            NativeSource::Molecule(mol) => {
                Ids::MoleculeAtoms(native::select(mol).atoms().ids().to_vec())
            }
            NativeSource::Protein(protein) => {
                Ids::ProteinAtoms(native::select(protein).atoms().ids().to_vec())
            }
        };
        Ok(Selection {
            source: self.source.clone(),
            ids,
        })
    }
    pub fn bonds(&self) -> PyResult<Selection> {
        let NativeSource::Molecule(mol) = &self.source.native else {
            return Err(value_error("protein selections cannot contain bonds"));
        };
        Ok(Selection {
            source: self.source.clone(),
            ids: Ids::MoleculeBonds(native::select(mol).bonds().ids().to_vec()),
        })
    }
    pub fn residues(&self) -> PyResult<Selection> {
        let NativeSource::Protein(protein) = &self.source.native else {
            return Err(value_error("molecule selections cannot contain residues"));
        };
        Ok(Selection {
            source: self.source.clone(),
            ids: Ids::ProteinResidues(native::select(protein).residues().ids().to_vec()),
        })
    }
}
// Reborrow an owned selection from its retained native source. Filtering cannot
// introduce IDs or rebind them to a different source; the core owns selection order.
fn subset<'a, S, I: Copy + Eq + std::hash::Hash>(
    selection: native::Selection<'a, S, I>,
    ids: &[I],
) -> native::Selection<'a, S, I> {
    let ids: HashSet<_> = ids.iter().copied().collect();
    selection.filter(|id| ids.contains(id))
}
impl Selection {
    pub(crate) fn native(&self) -> NativeSelection<'_> {
        match (&self.source.native, &self.ids) {
            (NativeSource::Molecule(mol), Ids::MoleculeAtoms(ids)) => {
                NativeSelection::MoleculeAtoms(subset(native::select(mol).atoms(), ids))
            }
            (NativeSource::Molecule(mol), Ids::MoleculeBonds(ids)) => {
                NativeSelection::MoleculeBonds(subset(native::select(mol).bonds(), ids))
            }
            (NativeSource::Protein(protein), Ids::ProteinAtoms(ids)) => {
                NativeSelection::ProteinAtoms(subset(native::select(protein).atoms(), ids))
            }
            (NativeSource::Protein(protein), Ids::ProteinResidues(ids)) => {
                NativeSelection::ProteinResidues(subset(native::select(protein).residues(), ids))
            }
            _ => unreachable!("selection kind is constructed from its native source"),
        }
    }
}
#[pymethods]
impl Selection {
    /// Return a filtered native selection; the original selection is reusable.
    pub(crate) fn element(&self, element: External<'_, ElementType>) -> PyResult<Self> {
        let n = element.as_bound().getattr("atomic_number")?.extract()?;
        let element =
            ck::Element::from_atomic_number(n).ok_or_else(|| value_error("invalid element"))?;
        let ids = match self.native() {
            NativeSelection::MoleculeAtoms(s) => {
                Ids::MoleculeAtoms(s.element(element).ids().to_vec())
            }
            NativeSelection::ProteinAtoms(s) => {
                Ids::ProteinAtoms(s.element(element).ids().to_vec())
            }
            _ => return Err(value_error("element filtering requires an atom selection")),
        };
        Ok(Self {
            source: self.source.clone(),
            ids,
        })
    }
    pub fn aromatic(&self) -> PyResult<Self> {
        let NativeSelection::MoleculeBonds(selection) = self.native() else {
            return Err(value_error(
                "aromatic filtering requires a molecule bond selection",
            ));
        };
        Ok(Self {
            source: self.source.clone(),
            ids: Ids::MoleculeBonds(selection.aromatic().ids().to_vec()),
        })
    }
    #[getter]
    pub fn ids(&self) -> Vec<usize> {
        match &self.ids {
            Ids::MoleculeAtoms(ids) => ids.iter().map(|id| id.index()).collect(),
            Ids::MoleculeBonds(ids) => ids.iter().map(|id| id.index()).collect(),
            Ids::ProteinAtoms(ids) => ids.iter().map(|id| id.index()).collect(),
            Ids::ProteinResidues(ids) => ids.iter().map(|id| id.index()).collect(),
        }
    }
    fn __len__(&self) -> usize {
        match &self.ids {
            Ids::MoleculeAtoms(ids) => ids.len(),
            Ids::MoleculeBonds(ids) => ids.len(),
            Ids::ProteinAtoms(ids) => ids.len(),
            Ids::ProteinResidues(ids) => ids.len(),
        }
    }
}

/// Typed binding input; all projection/default-selection policy lives in core Rust.
#[derive(FromPyObject)]
pub(crate) enum SceneInput<'py> {
    Shape(crate::input::ShapeInput),
    Selection(Selection),
    Source(ScientificInput<'py>),
}
impl SceneInput<'_> {
    pub(crate) fn with_native(
        self,
        operation: impl FnOnce(&dyn cosmol_viewer_core::display::SceneItem) -> PyResult<()>,
    ) -> PyResult<Option<Arc<ImportedSource>>> {
        match self {
            Self::Shape(shape) => {
                operation(&cosmol_viewer_core::utils::Shape::from(shape))?;
                Ok(None)
            }
            Self::Selection(selection) => {
                match selection.native() {
                    NativeSelection::MoleculeAtoms(s) => operation(&s),
                    NativeSelection::MoleculeBonds(s) => operation(&s),
                    NativeSelection::ProteinAtoms(s) => operation(&s),
                    NativeSelection::ProteinResidues(s) => operation(&s),
                }?;
                Ok(Some(selection.source))
            }
            Self::Source(source) => {
                let source = source.import()?;
                match &source.native {
                    NativeSource::Molecule(mol) => operation(mol),
                    NativeSource::Protein(protein) => operation(protein),
                }?;
                Ok(Some(source))
            }
        }
    }
}
