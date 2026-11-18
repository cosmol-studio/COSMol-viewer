//! CK owns protein chemistry; molecular glyph generation is shared with molecules.
use super::{DisplayError, input::MoleculeInput};
use cosmolkit as ck;
use std::sync::{Arc, Mutex, Weak};

pub(super) struct ProteinMolecule {
    source: ck::Protein,
    pub input: MoleculeInput,
}

static CACHE: Mutex<Vec<Weak<ProteinMolecule>>> = Mutex::new(Vec::new());

pub(super) fn shared(source: &ck::Protein) -> Result<Arc<ProteinMolecule>, DisplayError> {
    let mut cache = CACHE.lock().unwrap();
    cache.retain(|entry| entry.strong_count() != 0);
    if let Some(existing) = cache
        .iter()
        .filter_map(Weak::upgrade)
        .find(|entry| entry.source == *source)
    {
        return Ok(existing);
    }
    let molecule = source
        .to_molecule_with_params(&ck::BioMoleculeParams {
            sanitize: false,
            remove_hs: false,
            flavor: ck::PdbReadFlags::KEEP_ALTERNATE_AND_PSEUDO_ATOMS,
            proximity_bonding: true,
        })
        .map_err(|error| DisplayError::ProteinChemistry(error.to_string()))?;
    // With filtering and removal disabled CK converts BIO rows in encounter order.
    // Validate this invariant before treating BioAtomId as a molecular atom index.
    let coordinates = molecule.conformers_3d().first();
    if molecule.num_atoms() != source.num_atoms()
        || source.atoms().any(|atom| {
            let index = atom.id().index();
            molecule
                .atom(ck::AtomId::new(index))
                .is_none_or(|m| m.element() != atom.element())
                || coordinates
                    .and_then(|c| c.coordinates().get(index))
                    .copied()
                    != Some(atom.position())
        })
    {
        return Err(DisplayError::ProteinChemistry(
            "CK conversion changed protein atom order or coordinates".into(),
        ));
    }
    let value = Arc::new(ProteinMolecule {
        source: source.clone(),
        input: MoleculeInput::from(&molecule),
    });
    cache.push(Arc::downgrade(&value));
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        display::ProjectSource,
        selection::select,
        styles::{BallAndStick, Stick},
    };
    #[test]
    fn conversion_is_shared_and_released_with_the_last_display() {
        let protein =
            ck::Protein::from_mmcif(include_str!("../../../../cosmol_viewer/examples/6fi1.cif"))
                .unwrap();
        let ids = select(&protein).atoms().ids().to_vec();
        let first = protein
            .project(&ids, BallAndStick::default().into())
            .unwrap();
        let second = protein
            .project(&ids[..10], Stick::default().into())
            .unwrap();
        let a = first._protein_molecule.as_ref().unwrap();
        assert!(Arc::ptr_eq(a, second._protein_molecule.as_ref().unwrap()));
        let weak = Arc::downgrade(a);
        drop(first);
        assert!(weak.upgrade().is_some());
        drop(second);
        assert!(weak.upgrade().is_none());
    }
}
