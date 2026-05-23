use cosmol_viewer::{Scene, Viewer, shapes::Molecule, shapes::Protein};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prot = Protein::from_mmcif(include_str!("../examples/6fi1.cif"))?.rainbow_residues();
    let ligand = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?
        .set_outline(true, "#EEEEEE", 0.02);

    let mut scene = Scene::new();
    scene.recenter(ligand.get_center());
    scene.add_shape_with_id("prot", prot);
    scene.add_shape_with_id("ligand", ligand);
    scene.set_background_color("#021529");

    Viewer::render(&scene, 800.0, 500.0)?;

    println!("Press Enter to exit...");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
