use cosmol_viewer_core::{
    display::DisplayError,
    scene::Scene,
    selection::select,
    styles::{BallAndStick, Cartoon, Coordinates, Sphere, Stick, Surface},
    utils::{Interpolatable, RustLogger},
};
use cosmolkit as ck;

fn molecule(smiles: &str, xyz: Vec<[f64; 3]>) -> ck::Molecule {
    let source = ck::Molecule::from_smiles(smiles).unwrap();
    ck::Molecule::from_parts(
        source.topology().clone(),
        ck::CoordinateBlock {
            conformers_3d: vec![ck::Conformer3D::new(7, xyz, true)],
            ..Default::default()
        },
        source.properties().clone(),
    )
    .unwrap()
}

fn carbon_oxygen() -> ck::Molecule {
    molecule("CO", vec![[1.0, 2.0, 3.0], [2.2, 2.0, 3.0]])
}

const PROTEIN_PDB: &str = concat!(
    "ATOM      1  N   ALA A   1      11.104  13.207   9.900  1.00 20.00           N\n",
    "ATOM      2  CA  ALA A   1      12.210  13.912  10.555  1.00 20.00           C\n",
    "ATOM      3  C   ALA A   1      13.470  13.079  10.413  1.00 20.00           C\n",
    "ATOM      4  O   ALA A   1      14.000  12.500  11.000  1.00 20.00           O\n",
    "ATOM      5  N   GLY A   2      14.000  13.200  10.500  1.00 20.00           N\n",
    "ATOM      6  CA  GLY A   2      15.200  13.900  10.200  1.00 20.00           C\n",
    "ATOM      7  C   GLY A   2      16.300  13.100  10.600  1.00 20.00           C\n",
    "ATOM      8  O   GLY A   2      17.000  12.700  11.100  1.00 20.00           O\n",
);

#[test]
fn independent_displays_use_selected_entities_and_do_not_mutate_source() {
    let source = carbon_oxygen();
    let before: Vec<_> = source.conformers_3d()[0].coordinates().to_vec();
    let mut scene = Scene::new();
    scene
        .show(&source, &select(&source).atoms(), BallAndStick::default())
        .unwrap();
    let first = serde_json::to_value(&scene).unwrap();
    let oxygen = select(&source).atoms().element(ck::Element::O);
    scene
        .show(
            &source,
            &oxygen,
            Sphere::default().scale(1.4).color("#00FF00"),
        )
        .unwrap();
    assert_eq!(scene.unnamed_shapes.len(), 2);
    assert_eq!(
        serde_json::to_value(&scene).unwrap()["unnamed_shapes"][0],
        first["unnamed_shapes"][0]
    );
    let groups = scene.get_instances_grouped();
    assert_eq!(groups.spheres.len(), 3);
    assert_eq!(groups.sticks.len(), 2);
    assert_eq!(groups.spheres[2].position, [2.2, 2.0, 3.0]);
    assert_eq!(groups.spheres[2].color, [0.0, 1.0, 0.0, 1.0]);
    assert!((groups.spheres[2].radius - 1.52 * 1.4).abs() < 1e-6);
    assert_eq!(source.conformers_3d()[0].coordinates(), before);
}

#[test]
fn failure_is_atomic_for_named_and_unnamed_displays() {
    let source = carbon_oxygen();
    let other = carbon_oxygen();
    let mut scene = Scene::new();
    scene
        .show_with_id(
            "keep",
            &source,
            &select(&source).atoms(),
            BallAndStick::default(),
        )
        .unwrap();
    let before = serde_json::to_value(&scene).unwrap();
    assert!(matches!(
        scene.show(&other, &select(&source).atoms(), BallAndStick::default()),
        Err(DisplayError::SourceMismatch)
    ));
    assert!(matches!(
        scene.show_with_id(
            "keep",
            &other,
            &select(&source).atoms(),
            BallAndStick::default()
        ),
        Err(DisplayError::SourceMismatch)
    ));
    assert!(matches!(
        scene.show_with_id(
            "keep",
            &source,
            &select(&source).atoms(),
            Cartoon::default()
        ),
        Err(DisplayError::IncompatibleStyle { .. })
    ));
    assert!(matches!(
        scene.show(&source, &select(&source).bonds(), Sphere::default()),
        Err(DisplayError::IncompatibleStyle { .. })
    ));
    assert_eq!(serde_json::to_value(&scene).unwrap(), before);
}

#[test]
fn empty_compatible_selections_do_not_require_coordinates() {
    let source = ck::Molecule::new();
    let mut scene = Scene::new();
    scene
        .show(&source, &select(&source).atoms(), BallAndStick::default())
        .unwrap();
    scene
        .show(&source, &select(&source).bonds(), Stick::default())
        .unwrap();
    let groups = scene.get_instances_grouped();
    assert!(groups.spheres.is_empty() && groups.sticks.is_empty());
    assert_eq!(scene.unnamed_shapes.len(), 2);
}

#[test]
fn missing_coordinates_are_an_error_not_an_implicit_chemical_operation() {
    let source = ck::Molecule::from_smiles("CCO").unwrap();
    let mut scene = Scene::new();
    assert!(matches!(
        scene.show(&source, &select(&source).atoms(), BallAndStick::default()),
        Err(DisplayError::MissingCoordinates)
    ));
    assert!(scene.unnamed_shapes.is_empty());
    assert!(source.conformers_3d().is_empty() && source.coordinates_2d().is_none());
}

#[test]
fn coordinate_choices_use_ids_and_preserve_dimension() {
    let base = carbon_oxygen();
    let source = ck::Molecule::from_parts(
        base.topology().clone(),
        ck::CoordinateBlock {
            conformers_2d: vec![ck::Conformer2D::new(7, vec![[10.0, 20.0], [30.0, 40.0]])],
            conformers_3d: vec![
                ck::Conformer3D::new(7, vec![[1.0; 3], [2.0; 3]], true),
                ck::Conformer3D::new(42, vec![[4.0; 3], [5.0; 3]], true),
            ],
            ..Default::default()
        },
        base.properties().clone(),
    )
    .unwrap();
    let mut scene = Scene::new();
    assert!(matches!(
        scene.show(&source, &select(&source).atoms(), BallAndStick::default()),
        Err(DisplayError::AmbiguousCoordinates(2))
    ));
    assert!(matches!(
        scene.show(
            &source,
            &select(&source).atoms(),
            BallAndStick::default().coordinates(Coordinates::ThreeD { id: 1 })
        ),
        Err(DisplayError::MissingConformer(1))
    ));
    assert!(scene.unnamed_shapes.is_empty());
    scene
        .show(
            &source,
            &select(&source).atoms(),
            Sphere::default().coordinates(Coordinates::ThreeD { id: 42 }),
        )
        .unwrap();
    scene
        .show(
            &source,
            &select(&source).atoms(),
            Sphere::default().coordinates(Coordinates::TwoD),
        )
        .unwrap();
    let groups = scene.get_instances_grouped();
    assert_eq!(groups.spheres[0].position, [4.0; 3]);
    assert_eq!(groups.spheres[2].position, [10.0, 20.0, 0.0]);
}

#[test]
fn centering_and_scaling_change_visual_data_only() {
    let source = carbon_oxygen();
    let mut scene = Scene::new();
    scene
        .show_with_id(
            "ligand",
            &source,
            &select(&source).atoms(),
            BallAndStick::default().outline("#EEEEEE", 0.02),
        )
        .unwrap();
    scene.recenter_on(&source).unwrap();
    assert_eq!(scene.scene_center, [1.6, 2.0, 3.0]);
    scene.set_scale(2.0);
    let groups = scene.get_instances_grouped();
    assert_eq!(groups.spheres[0].position, [2.0, 4.0, 6.0]);
    assert_eq!(groups.outlines[0].settings.width, 0.04);
    assert_eq!(
        groups.outlines[0].spheres[0].position,
        groups.spheres[0].position
    );
    assert_eq!(source.conformers_3d()[0].coordinates()[0], [1.0, 2.0, 3.0]);
}

#[test]
fn scientific_sources_can_be_dropped_after_show_and_transport_contains_only_geometry() {
    let mut scene = Scene::new();
    {
        let source = carbon_oxygen();
        scene
            .show(&source, &select(&source).atoms(), BallAndStick::default())
            .unwrap();
    }
    let json = serde_json::to_string(&scene).unwrap();
    for scientific_field in ["atom_types", "bond_types", "conformers_3d", "residue_type"] {
        assert!(!json.contains(scientific_field));
    }
    let decoded: Scene = serde_json::from_str(&json).unwrap();
    let bytes = postcard::to_allocvec(&scene).unwrap();
    let postcard_scene: Scene = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        serde_json::to_value(&scene).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&postcard_scene).unwrap(),
        serde_json::to_value(&scene).unwrap()
    );
    assert_eq!(decoded.get_instances_grouped().spheres.len(), 2);
}

#[test]
fn protein_cartoon_and_atom_displays_share_one_ck_source() {
    let source = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    let before: Vec<_> = source.atoms().map(|a| a.position()).collect();
    let mut scene = Scene::new();
    scene
        .show(
            &source,
            &select(&source).residues(),
            Cartoon::default().rainbow_residues(),
        )
        .unwrap();
    scene
        .show(
            &source,
            &select(&source).atoms().element(ck::Element::O),
            Sphere::default(),
        )
        .unwrap();
    assert_eq!(scene.unnamed_shapes.len(), 2);
    assert_eq!(scene.get_instances_grouped().spheres.len(), 2);
    let meshes = scene._get_meshes();
    assert!(!meshes[0].vertices.is_empty());
    assert!(meshes[0].vertices.iter().all(|v| v.is_finite()));
    assert_eq!(meshes[0].vertices.len(), meshes[0].normals.len());
    assert_eq!(
        source.atoms().map(|a| a.position()).collect::<Vec<_>>(),
        before
    );
}

#[test]
fn surface_errors_do_not_replace_a_previous_display() {
    let source = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    let mut scene = Scene::new();
    scene
        .show_with_id(
            "protein",
            &source,
            &select(&source).residues(),
            Cartoon::default(),
        )
        .unwrap();
    let before = serde_json::to_value(&scene).unwrap();
    let mut invalid = Surface::default();
    invalid.grid_spacing = 0.0;
    assert!(matches!(
        scene.show_with_id("protein", &source, &select(&source).atoms(), invalid),
        Err(DisplayError::Surface(_))
    ));
    assert_eq!(serde_json::to_value(&scene).unwrap(), before);
}

#[test]
fn protein_surface_produces_finite_render_geometry() {
    let source = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    let mut scene = Scene::new();
    let mut style = Surface::default().color("#DCE8F2").opacity(0.9);
    style.sharp_boundaries = false;
    scene
        .show(&source, &select(&source).atoms(), style)
        .unwrap();
    let meshes = scene._get_meshes();
    assert!(!meshes[0].vertices.is_empty());
    assert!(meshes[0].vertices.iter().all(|v| v.is_finite()));
    assert_eq!(meshes[0].vertices.len(), meshes[0].normals.len());
    assert_eq!(meshes[0].colors.as_ref().unwrap()[0].w, 0.9);
}

#[test]
fn zero_length_neighbor_bond_does_not_hide_valid_bond() {
    let source = molecule("COC", vec![[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]]);
    let mut scene = Scene::new();
    scene
        .show(&source, &select(&source).atoms(), BallAndStick::default())
        .unwrap();
    let groups = scene.get_instances_grouped();
    assert_eq!(groups.sticks.len(), 2);
    assert!(
        groups
            .sticks
            .iter()
            .all(|s| s.start.iter().chain(&s.end).all(|x| x.is_finite()))
    );
}

#[test]
fn stick_style_preserves_double_and_triple_order_spacing() {
    let source = molecule(
        "O=C=NC#N",
        vec![
            [0.0; 3],
            [1.2, 0.0, 0.0],
            [2.4, 0.0, 0.0],
            [3.6, 0.0, 0.0],
            [4.8, 0.0, 0.0],
        ],
    );
    let mut scene = Scene::new();
    scene
        .show(&source, &select(&source).bonds(), Stick::default())
        .unwrap();
    let groups = scene.get_instances_grouped();
    // two double, one single and one triple bond, each colored in two halves.
    assert_eq!(groups.sticks.len(), 16);
    assert_eq!(groups.spheres.len(), 16);
    assert!(groups.sticks.iter().any(|s| (s.radius - 0.1).abs() < 1e-6));
    assert!(
        groups
            .sticks
            .iter()
            .any(|s| (s.radius - 0.2 / 3.0).abs() < 1e-6)
    );
}

#[test]
fn aromatic_ring_keeps_inner_lines_and_bond_selection_has_no_unselected_bonds() {
    let mut xyz: Vec<_> = (0..6)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::TAU / 6.0;
            [angle.cos(), angle.sin(), 0.0]
        })
        .collect();
    xyz.push([2.0, 0.0, 0.0]);
    xyz.push([3.0, 0.0, 0.0]);
    let source = molecule("c1ccccc1CO", xyz);
    let mut scene = Scene::new();
    scene
        .show(
            &source,
            &select(&source).bonds().aromatic(),
            Stick::default(),
        )
        .unwrap();
    let groups = scene.get_instances_grouped();
    assert_eq!(groups.sticks.len(), 24);
    assert_eq!(groups.spheres.len(), 24);
}

#[test]
fn interpolation_operates_on_visual_geometry() {
    let a = carbon_oxygen();
    let b = molecule("CO", vec![[3.0, 2.0, 3.0], [4.2, 2.0, 3.0]]);
    let mut first = Scene::new();
    let mut second = Scene::new();
    first
        .show_with_id("mol", &a, &select(&a).atoms(), BallAndStick::default())
        .unwrap();
    second
        .show_with_id("mol", &b, &select(&b).atoms(), BallAndStick::default())
        .unwrap();
    let midway = first.interpolate(&second, 0.5, RustLogger);
    assert_eq!(
        midway.get_instances_grouped().spheres[0].position,
        [2.0, 2.0, 3.0]
    );
}

#[test]
fn source_properties_color_original_atom_ids_not_selection_offsets() {
    let base = molecule("CCO", vec![[0.0; 3], [1.4, 0.0, 0.0], [2.8, 0.0, 0.0]]);
    for properties in [
        base.properties()
            .clone()
            .with_sdf_data_field("DUMMY_WEIGHTS", "0.25\n0.5\n0.75"),
        base.properties()
            .clone()
            .with_sdf_data_field("DUMMY_ATOM_INDICES", "3")
            .with_sdf_data_field("DUMMY_WEIGHTS", "0.75"),
    ] {
        let source = ck::Molecule::from_parts(
            base.topology().clone(),
            ck::CoordinateBlock {
                conformers_3d: base.conformers_3d().to_vec(),
                ..Default::default()
            },
            properties,
        )
        .unwrap();
        let mut scene = Scene::new();
        scene
            .show(
                &source,
                &select(&source).atoms().element(ck::Element::O),
                Sphere::default(),
            )
            .unwrap();
        let groups = scene.get_instances_grouped();
        assert_eq!(groups.spheres.len(), 1);
        assert_eq!(groups.spheres[0].color, [0.75, 0.25, 0.0, 1.0]);
    }
}

#[test]
fn oversized_f64_coordinates_are_rejected_before_scene_mutation() {
    let source = molecule("CO", vec![[f64::MAX, 0.0, 0.0], [0.0; 3]]);
    let mut scene = Scene::new();
    assert!(matches!(
        scene.show(&source, &select(&source).atoms(), BallAndStick::default()),
        Err(DisplayError::InvalidCoordinates)
    ));
    assert!(scene.unnamed_shapes.is_empty());
}

#[test]
fn projection_extension_validates_entity_ids() {
    use cosmol_viewer_core::display::ProjectSource;
    let source = carbon_oxygen();
    assert!(matches!(
        source.project(&[ck::AtomId::new(999)], BallAndStick::default().into()),
        Err(DisplayError::InvalidEntity { .. })
    ));
    assert!(matches!(
        source.project(&[ck::BondId::new(999)], Stick::default().into()),
        Err(DisplayError::InvalidEntity { .. })
    ));
    let protein = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    assert!(matches!(
        protein.project(&[ck::BioAtomId::new(999)], Sphere::default().into()),
        Err(DisplayError::InvalidEntity { .. })
    ));
    assert!(matches!(
        protein.project(&[ck::BioResidueId::new(999)], Cartoon::default().into()),
        Err(DisplayError::InvalidEntity { .. })
    ));
}

#[test]
fn changing_protein_mesh_topology_keeps_discrete_animation_playback() {
    let first_source = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    let second_source =
        ck::Protein::from_mmcif(include_str!("../../../cosmol_viewer/examples/6fi1.cif")).unwrap();
    let mut first = Scene::new();
    let mut second = Scene::new();
    first
        .show_with_id(
            "protein",
            &first_source,
            &select(&first_source).residues(),
            Cartoon::default(),
        )
        .unwrap();
    second
        .show_with_id(
            "protein",
            &second_source,
            &select(&second_source).residues(),
            Cartoon::default(),
        )
        .unwrap();
    assert_ne!(
        first._get_meshes()[0].vertices.len(),
        second._get_meshes()[0].vertices.len()
    );
    let midway = first.interpolate(&second, 0.5, RustLogger);
    assert_eq!(
        serde_json::to_value(&midway).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
}

#[test]
fn recenter_on_molecule_works_without_a_display_or_trait_import() {
    let source = carbon_oxygen();
    let before = source.conformers_3d()[0].coordinates().to_vec();
    let mut scene = Scene::new();
    scene.recenter_on(&source).unwrap();
    assert_eq!(scene.scene_center, [1.6, 2.0, 3.0]);
    assert!(scene.named_shapes.is_empty() && scene.unnamed_shapes.is_empty());
    assert_eq!(source.conformers_3d()[0].coordinates(), before);
}

#[test]
fn recenter_on_protein_uses_all_atoms_without_changing_coordinates() {
    let protein = ck::Protein::from_pdb(PROTEIN_PDB).unwrap();
    let before: Vec<_> = protein.atoms().map(|atom| atom.position()).collect();
    let mut scene = Scene::new();
    scene.recenter_on(&protein).unwrap();
    for axis in 0..3 {
        let expected = before.iter().map(|p| p[axis]).sum::<f64>() / before.len() as f64;
        assert!((scene.scene_center[axis] - expected as f32).abs() < 1.0e-5);
    }
    assert!(scene.named_shapes.is_empty() && scene.unnamed_shapes.is_empty());
    assert_eq!(
        protein
            .atoms()
            .map(|atom| atom.position())
            .collect::<Vec<_>>(),
        before
    );
}

#[test]
fn recenter_on_errors_leave_the_scene_unchanged() {
    let mut scene = Scene::new();
    scene.recenter([9.0, 8.0, 7.0]);
    let before = serde_json::to_value(&scene).unwrap();
    let missing = ck::Molecule::from_smiles("CO").unwrap();
    assert!(matches!(
        scene.recenter_on(&missing),
        Err(DisplayError::MissingCoordinates)
    ));
    assert_eq!(serde_json::to_value(&scene).unwrap(), before);

    let base = carbon_oxygen();
    let ambiguous = ck::Molecule::from_parts(
        base.topology().clone(),
        ck::CoordinateBlock {
            conformers_3d: vec![
                ck::Conformer3D::new(7, vec![[1.0; 3], [2.0; 3]], true),
                ck::Conformer3D::new(42, vec![[3.0; 3], [4.0; 3]], true),
            ],
            ..Default::default()
        },
        base.properties().clone(),
    )
    .unwrap();
    assert!(matches!(
        scene.recenter_on(&ambiguous),
        Err(DisplayError::AmbiguousCoordinates(2))
    ));
    assert_eq!(serde_json::to_value(&scene).unwrap(), before);

    struct NonFiniteCenter;
    impl cosmol_viewer_core::display::CenterSource for NonFiniteCenter {
        fn center(&self) -> Result<[f32; 3], DisplayError> {
            Ok([f32::NAN; 3])
        }
    }
    assert!(matches!(
        scene.recenter_on(&NonFiniteCenter),
        Err(DisplayError::InvalidCoordinates)
    ));
    assert_eq!(serde_json::to_value(&scene).unwrap(), before);
}

#[test]
fn recenter_on_empty_molecule_uses_origin_without_generating_coordinates() {
    let empty = ck::Molecule::new();
    let mut scene = Scene::new();
    scene.recenter([9.0; 3]);
    scene.recenter_on(&empty).unwrap();
    assert_eq!(scene.scene_center, [0.0; 3]);
    assert!(empty.coordinates_2d().is_none() && empty.conformers_3d().is_empty());
}
