use cosmol_viewer::utils::VisualShape;
use cosmol_viewer::{Scene, Viewer, shapes::Molecule, shapes::Protein};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prot = Protein::from_mmcif(include_str!("../examples/6fi1.cif"))?.color("#10ACBF");
    let ligand = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?;

    let mut scene = Scene::new();
    scene.recenter(ligand.get_center());
    scene.add_shape_with_id("prot", prot);
    scene.add_shape_with_id("ligand", ligand);

    Viewer::render(&scene, 800.0, 500.0)?;

    println!("Press Enter to exit...");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
