use cosmol_viewer::{Scene, Viewer, shapes::Molecule, shapes::Protein};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let prot = Protein::from_mmcif(include_str!("../examples/6fi1.cif"))?.rainbow_residues();
    let ligand = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?
        .set_outline(true, "#EEEEEE", 0.02);

    let mut scene = Scene::new();
    scene.recenter(ligand.get_center());
    scene.add_shape_with_id("prot", prot);
    scene.add_shape_with_id("ligand", ligand);
    scene.set_background_color("#021529");

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;

    viewer.keep_alive()?;

    Ok(())
}
