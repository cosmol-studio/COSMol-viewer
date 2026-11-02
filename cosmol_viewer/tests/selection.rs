use cosmol_viewer::{Selectable, cosmolkit as ck, select};

const PROTEIN_PDB: &str = concat!(
    "ATOM      1  CA  ALA A   1       1.000   2.000   3.000  1.00 20.00           C  \n",
    "ATOM      2  O   ALA A   1       2.000   2.000   3.000  1.00 20.00           O  \n",
    "ATOM      3  CA  GLY A   2       4.000   2.000   3.000  1.00 20.00           C  \n",
    "ATOM      4  O   GLY A   2       5.000   2.000   3.000  1.00 20.00           O  \n",
);

fn generic_select<S: Selectable>(source: &S) -> S::Selection<'_> {
    select(source)
}

#[test]
fn molecule_atoms_filter_without_changing_collection_access() {
    let mol = ck::Molecule::from_smiles("CCO").unwrap();
    let all = generic_select(&mol).atoms();
    assert_eq!(all.len(), 3);
    assert_eq!(
        all.ids(),
        &[ck::AtomId::new(0), ck::AtomId::new(1), ck::AtomId::new(2)]
    );
    assert!(std::ptr::eq(all.source(), &mol));

    let oxygen = select(&mol).atoms().element(ck::Element::O);
    assert_eq!(oxygen.ids(), &[ck::AtomId::new(2)]);
    assert!(oxygen.belongs_to(&mol));
    assert_eq!(mol.atoms().len(), 3);
    assert_eq!(
        mol.atom(ck::AtomId::new(2)).unwrap().element(),
        ck::Element::O
    );
    assert!(select(&mol).atoms().element(ck::Element::F).is_empty());
    assert!(oxygen.clone().element(ck::Element::C).is_empty());
    assert_eq!(oxygen.len(), 1);
    assert_eq!(all.len(), 3);
}

#[test]
fn molecule_bonds_filter_existing_aromatic_flags() {
    let mol = ck::Molecule::from_smiles("c1ccccc1CO").unwrap();
    let all = select(&mol).bonds();
    assert_eq!(all.len(), mol.num_bonds());
    let aromatic = select(&mol).bonds().aromatic();
    let expected: Vec<_> = mol
        .bonds()
        .iter()
        .filter(|bond| bond.is_aromatic())
        .map(|bond| bond.id())
        .collect();
    assert_eq!(aromatic.ids(), expected);
    assert_eq!(aromatic.len(), 6);
    assert!(aromatic.belongs_to(&mol));
    assert_eq!(select(&mol).bonds().len(), all.len());
}

#[test]
fn molecule_identity_rejects_equal_and_derived_sources() {
    let mol = ck::Molecule::from_smiles("CCO").unwrap();
    let same_contents = ck::Molecule::from_smiles("CCO").unwrap();
    let cloned = mol.clone();
    let hydrogenated = mol.with_hydrogens().unwrap();
    let oxygen = select(&mol).atoms().element(ck::Element::O);
    assert!(oxygen.belongs_to(&mol));
    assert!(!oxygen.belongs_to(&same_contents));
    assert!(!oxygen.belongs_to(&cloned));
    assert!(!oxygen.belongs_to(&hydrogenated));
}

#[test]
fn protein_atoms_and_residues_retain_the_original_source() {
    let protein = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    let all = generic_select(&protein).atoms();
    assert_eq!(all.len(), 4);
    assert_eq!(
        all.ids(),
        &[
            ck::BioAtomId::new(0),
            ck::BioAtomId::new(1),
            ck::BioAtomId::new(2),
            ck::BioAtomId::new(3)
        ]
    );
    let oxygen = select(&protein).atoms().element(ck::Element::O);
    assert_eq!(
        oxygen.ids(),
        &[ck::BioAtomId::new(1), ck::BioAtomId::new(3)]
    );
    assert!(oxygen.belongs_to(&protein));
    assert!(std::ptr::eq(oxygen.source(), &protein));
    assert!(oxygen.clone().element(ck::Element::C).is_empty());
    assert!(select(&protein).atoms().element(ck::Element::F).is_empty());

    let residues = select(&protein).residues();
    assert_eq!(
        residues.ids(),
        &[ck::BioResidueId::new(0), ck::BioResidueId::new(1)]
    );
    assert_eq!(residues.len(), protein.num_residues());
    assert!(residues.belongs_to(&protein));
    assert!(!oxygen.belongs_to(&protein.clone()));
    assert_eq!(protein.atoms().count(), 4);
}

#[test]
fn empty_source_selections_are_valid() {
    let mol = ck::Molecule::new();
    assert!(select(&mol).atoms().element(ck::Element::O).is_empty());
    assert!(select(&mol).bonds().aromatic().is_empty());
    assert!(select(&mol).atoms().belongs_to(&mol));
}
