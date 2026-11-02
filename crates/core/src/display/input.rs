//! Read-only rendering inputs for bindings in separate runtimes.
//! These temporary arrays cannot edit or reconstruct scientific objects.
use super::{Display, DisplayError, center, position};
use crate::styles::{Coordinates, Representation};
use crate::utils::MeshData;
use cosmolkit as ck;
use glam::Vec3;

#[derive(Clone, PartialEq)]
pub struct BondInput {
    pub begin: usize,
    pub end: usize,
    pub order: ck::BondOrder,
    pub aromatic: bool,
}
#[derive(Clone, PartialEq)]
pub struct ConformerInput {
    pub id: usize,
    pub coordinates: Vec<[f64; 3]>,
}
#[derive(Clone, PartialEq)]
pub struct MoleculeInput {
    pub elements: Vec<ck::Element>,
    pub bonds: Vec<BondInput>,
    pub conformers: Vec<ConformerInput>,
    pub coordinates_2d: Option<Vec<[f64; 2]>>,
    pub atom_weights: Vec<Option<f64>>,
    pub dummy_weights: Option<String>,
    pub dummy_indices: Option<String>,
}
impl From<&ck::Molecule> for MoleculeInput {
    fn from(source: &ck::Molecule) -> Self {
        let field = |name: &str| {
            source
                .properties()
                .sdf_data_fields()
                .iter()
                .rev()
                .find(|(k, _)| k.as_bytes().eq_ignore_ascii_case(name.as_bytes()))
                .map(|(_, v)| String::from_utf8_lossy(v.as_bytes()).into_owned())
        };
        Self {
            elements: source.atoms().iter().map(|a| a.element()).collect(),
            bonds: source
                .bonds()
                .iter()
                .map(|b| BondInput {
                    begin: b.begin().index(),
                    end: b.end().index(),
                    order: b.order(),
                    aromatic: b.is_aromatic(),
                })
                .collect(),
            conformers: source
                .conformers_3d()
                .iter()
                .map(|c| ConformerInput {
                    id: c.id(),
                    coordinates: c.coordinates().to_vec(),
                })
                .collect(),
            coordinates_2d: source.coordinates_2d().map(<[_]>::to_vec),
            atom_weights: source
                .atoms()
                .iter()
                .map(|a| a.prop("WEIGHT").and_then(|v| v.as_double().ok()))
                .collect(),
            dummy_weights: field("DUMMY_WEIGHTS"),
            dummy_indices: field("DUMMY_ATOM_INDICES"),
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct ProteinAtomInput {
    pub id: ck::BioAtomId,
    pub name: String,
    pub position: Vec3,
    /// Preserve the original precision for foreign-source mutation checks.
    pub source_position: [f64; 3],
    pub atomic_number: u8,
    pub alt_loc: Option<u8>,
    pub occupancy: f64,
    pub b_factor: f64,
}
#[derive(Clone, PartialEq)]
pub struct ProteinResidueInput {
    pub id: ck::BioResidueId,
    pub name: String,
    pub residue_type: ck::ResidueCode,
    pub atoms: Vec<ProteinAtomInput>,
    pub seq_id: Option<(i32, Option<u8>)>,
    pub label_seq_id: Option<i32>,
}
#[derive(Clone, PartialEq)]
pub struct ProteinInput {
    pub chains: Vec<Vec<ProteinResidueInput>>,
    pub input_format: ck::BioCoordinateFormat,
}
impl TryFrom<&ck::Protein> for ProteinInput {
    type Error = DisplayError;
    fn try_from(source: &ck::Protein) -> Result<Self, Self::Error> {
        Ok(Self {
            chains: protein_chains(source)?,
            input_format: source.input_format(),
        })
    }
}
fn protein_chains(protein: &ck::Protein) -> Result<Vec<Vec<ProteinResidueInput>>, DisplayError> {
    protein
        .chains()
        .map(|chain| {
            chain
                .residues()
                .map(|residue| {
                    let atoms = residue
                        .atoms()
                        .map(|atom| {
                            Ok(ProteinAtomInput {
                                id: atom.id(),
                                name: atom.name().as_str().trim().to_owned(),
                                position: position(atom.position())?,
                                source_position: atom.position(),
                                atomic_number: atom.element().atomic_number(),
                                alt_loc: atom.row().altloc().map(|a| a.value()),
                                occupancy: atom.row().occupancy(),
                                b_factor: atom.row().b_iso(),
                            })
                        })
                        .collect::<Result<Vec<_>, DisplayError>>()?;
                    Ok(ProteinResidueInput {
                        id: residue.id(),
                        name: residue.name().as_str().to_owned(),
                        residue_type: residue.code(),
                        atoms,
                        seq_id: residue
                            .row()
                            .source()
                            .seq_id()
                            .map(|s| (s.seq_num(), s.ins_code())),
                        label_seq_id: residue.row().source().label_seq_id(),
                    })
                })
                .collect()
        })
        .collect()
}

/// Captured renderer inputs, retained by binding selectors rather than Scene.
#[derive(Clone, PartialEq)]
pub enum SourceInput {
    Molecule(MoleculeInput),
    Protein(ProteinInput),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Atoms,
    Bonds,
    Residues,
}
impl SourceInput {
    pub fn ids(&self, kind: EntityKind) -> Result<Vec<usize>, DisplayError> {
        Ok(match (self, kind) {
            (Self::Molecule(m), EntityKind::Atoms) => (0..m.elements.len()).collect(),
            (Self::Molecule(m), EntityKind::Bonds) => (0..m.bonds.len()).collect(),
            (Self::Protein(p), EntityKind::Atoms) => p
                .chains
                .iter()
                .flatten()
                .flat_map(|r| &r.atoms)
                .map(|a| a.id.index())
                .collect(),
            (Self::Protein(p), EntityKind::Residues) => {
                p.chains.iter().flatten().map(|r| r.id.index()).collect()
            }
            _ => return Err(DisplayError::InvalidSelection),
        })
    }
    pub fn filter_element(
        &self,
        kind: EntityKind,
        ids: &mut Vec<usize>,
        element: ck::Element,
    ) -> Result<(), DisplayError> {
        if kind != EntityKind::Atoms {
            return Err(DisplayError::InvalidSelection);
        }
        match self {
            Self::Molecule(m) => ids.retain(|&i| m.elements.get(i) == Some(&element)),
            Self::Protein(p) => {
                let allowed: std::collections::BTreeSet<_> = p
                    .chains
                    .iter()
                    .flatten()
                    .flat_map(|r| &r.atoms)
                    .filter(|a| a.atomic_number == element.atomic_number())
                    .map(|a| a.id.index())
                    .collect();
                ids.retain(|i| allowed.contains(i));
            }
        }
        Ok(())
    }
    pub fn filter_aromatic(
        &self,
        kind: EntityKind,
        ids: &mut Vec<usize>,
    ) -> Result<(), DisplayError> {
        match self {
            Self::Molecule(m) if kind == EntityKind::Bonds => {
                ids.retain(|&i| m.bonds.get(i).is_some_and(|b| b.aromatic));
                Ok(())
            }
            _ => Err(DisplayError::InvalidSelection),
        }
    }
    pub fn center(&self) -> Result<[f32; 3], DisplayError> {
        let positions = match self {
            Self::Molecule(m) if m.elements.is_empty() => return Ok([0.; 3]),
            Self::Molecule(m) => super::molecular::coordinates(m, Coordinates::Auto)?,
            Self::Protein(p) => p
                .chains
                .iter()
                .flatten()
                .flat_map(|r| &r.atoms)
                .map(|a| a.position)
                .collect(),
        };
        if positions.iter().any(|p| !p.is_finite()) {
            return Err(DisplayError::InvalidCoordinates);
        }
        Ok(center(positions.into_iter()).to_array())
    }
    pub fn project(
        &self,
        kind: EntityKind,
        ids: &[usize],
        style: Representation,
    ) -> Result<Display, DisplayError> {
        let valid: std::collections::BTreeSet<_> = self.ids(kind)?.into_iter().collect();
        if let Some(&index) = ids.iter().find(|i| !valid.contains(i)) {
            return Err(DisplayError::InvalidEntity {
                kind: "selection entity",
                index,
            });
        }
        if let Self::Molecule(m) = self {
            if let Some(index) = m
                .bonds
                .iter()
                .flat_map(|b| [b.begin, b.end])
                .find(|&i| i >= m.elements.len())
            {
                return Err(DisplayError::InvalidEntity {
                    kind: "bond endpoint",
                    index,
                });
            }
        }
        let (mesh, instances, center) = match (self, kind) {
            (Self::Molecule(m), EntityKind::Atoms) => {
                let (groups, center) = super::molecular::project_atoms(
                    m,
                    &ids.iter().copied().map(ck::AtomId::new).collect::<Vec<_>>(),
                    style,
                )?;
                (MeshData::default(), groups, center)
            }
            (Self::Molecule(m), EntityKind::Bonds) => {
                let (groups, center) = super::molecular::project_bonds(
                    m,
                    &ids.iter().copied().map(ck::BondId::new).collect::<Vec<_>>(),
                    style,
                )?;
                (MeshData::default(), groups, center)
            }
            (Self::Protein(p), EntityKind::Atoms) => super::protein::project_atoms(
                p,
                &ids.iter()
                    .map(|&i| ck::BioAtomId::new(i as u32))
                    .collect::<Vec<_>>(),
                style,
            )?,
            (Self::Protein(p), EntityKind::Residues) => super::protein::project_residues(
                p,
                &ids.iter()
                    .map(|&i| ck::BioResidueId::new(i as u32))
                    .collect::<Vec<_>>(),
                style,
            )?,
            _ => return Err(DisplayError::InvalidSelection),
        };
        Ok(Display {
            mesh,
            instances,
            center,
            source: None,
        })
    }
}
