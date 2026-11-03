use cosmol_viewer_core::parser::utils::Residue;
use cosmol_viewer_core::{scene::Scene, selection::select, styles::Cartoon};
use cosmolkit::Protein;
use cosmolkit::{self as ck, ResidueCode};
use glam::Vec3;

const PDB: &str = "\
ATOM      1  N   ALA A   1      11.104  13.207   9.900  1.00 20.00           N
ATOM      2  CA  ALA A   1      12.210  13.912  10.555  1.00 20.00           C
ATOM      3  C   ALA A   1      13.470  13.079  10.413  1.00 20.00           C
ATOM      4  O   ALA A   1      14.000  12.500  11.000  1.00 20.00           O
END
";

const MMCIF: &str = "\
data_demo
loop_
_atom_site.group_PDB
_atom_site.id
_atom_site.type_symbol
_atom_site.label_atom_id
_atom_site.label_alt_id
_atom_site.label_comp_id
_atom_site.label_asym_id
_atom_site.label_seq_id
_atom_site.Cartn_x
_atom_site.Cartn_y
_atom_site.Cartn_z
_atom_site.occupancy
_atom_site.B_iso_or_equiv
_atom_site.auth_seq_id
_atom_site.auth_comp_id
_atom_site.auth_asym_id
_atom_site.auth_atom_id
_atom_site.pdbx_PDB_model_num
ATOM 1 N N . MSE A 1 11.104 13.207 9.900 1.00 20.00 1 MSE A N 1
ATOM 2 C CA . MSE A 1 12.210 13.912 10.555 1.00 20.00 1 MSE A CA 1
ATOM 3 C C . MSE A 1 13.470 13.079 10.413 1.00 20.00 1 MSE A C 1
ATOM 4 O O . MSE A 1 14.000 12.500 11.000 1.00 20.00 1 MSE A O 1
";

fn residue(code: ResidueCode) -> Residue {
    Residue {
        residue_type: code,
        sns: 1,
        c: Vec3::X,
        n: Vec3::Y,
        ca: Vec3::ZERO,
        o: Vec3::Z,
        h: None,
        ss: None,
    }
}

#[test]
fn pdb_keeps_ck_standard_and_modified_residue_codes() {
    for (name, expected) in [
        ("ALA", ResidueCode::ALA),
        ("PRO", ResidueCode::PRO),
        ("MSE", ResidueCode::MSE),
        ("HYP", ResidueCode::HYP),
        ("DPR", ResidueCode::DPR),
    ] {
        let protein = Protein::from_pdb(&PDB.replace("ALA", name)).unwrap();
        let mut scene = Scene::new();
        scene
            .add((&select(&protein).residues(), Cartoon::default()))
            .unwrap();
        assert_eq!(protein.chains().count(), 1, "{name}");
        assert_eq!(protein.residues().count(), 1, "{name}");
        assert_eq!(protein.residues().next().unwrap().code(), expected);
    }
}

#[test]
fn mmcif_keeps_modified_residue_identity() {
    let protein = Protein::from_mmcif(MMCIF).unwrap();
    let mut scene = Scene::new();
    scene
        .add((&select(&protein).residues(), Cartoon::default()))
        .unwrap();
    assert_eq!(protein.chains().count(), 1);
    assert_eq!(protein.residues().count(), 1);
    assert_eq!(protein.residues().next().unwrap().code(), ResidueCode::MSE);
}

#[test]
fn temporary_serde_uses_residue_names_without_collapsing_modifications() {
    for (code, name) in [
        (ResidueCode::MET, "MET"),
        (ResidueCode::MSE, "MSE"),
        (ResidueCode::PRO, "PRO"),
        (ResidueCode::HYP, "HYP"),
        (ResidueCode::R0TD, "0TD"),
        (ResidueCode::UNK, "UNK"),
        (ResidueCode::UNKNOWN, "UNKNOWN"),
    ] {
        let value = serde_json::to_value(residue(code)).unwrap();
        assert_eq!(value["residue_type"], name);
        let decoded: Residue = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.residue_type, code);
    }
}

#[test]
fn all_ck_residue_codes_round_trip_through_json_and_postcard() {
    for index in 0..=usize::from(ResidueCode::UNKNOWN.as_u16()) {
        let code = ck::residue_info_checked(index).unwrap().code;
        let original = residue(code);
        let json = serde_json::to_string(&original).unwrap();
        let decoded: Residue = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.residue_type, code, "JSON: {code:?}");
        let bytes = postcard::to_allocvec(&original).unwrap();
        let decoded: Residue = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.residue_type, code, "postcard: {code:?}");
    }
}

#[test]
fn temporary_serde_rejects_invalid_names_instead_of_guessing_unknown() {
    for name in ["", "not-a-residue", "NPRO", "CPRO"] {
        let mut value = serde_json::to_value(residue(ResidueCode::ALA)).unwrap();
        value["residue_type"] = serde_json::Value::String(name.to_string());
        let error = serde_json::from_value::<Residue>(value).unwrap_err();
        assert!(
            error.to_string().contains("Invalid CK residue name"),
            "{name}"
        );
    }
}

#[test]
fn scene_postcard_round_trip_transports_render_data_not_a_scientific_protein() {
    let protein =
        Protein::from_mmcif(include_str!("../../../cosmol_viewer/examples/6fi1.cif")).unwrap();
    let mut scene = Scene::new();
    scene
        .add((
            &select(&protein).residues(),
            Cartoon::default().rainbow_residues(),
        ))
        .unwrap();
    let bytes = postcard::to_allocvec(&scene).unwrap();
    let decoded: Scene = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        serde_json::to_value(&scene).unwrap()
    );
    // Modified residue names remain CK-owned; only derived render geometry travels.
    assert!(
        !serde_json::to_string(&scene)
            .unwrap()
            .contains("residue_type")
    );
}
