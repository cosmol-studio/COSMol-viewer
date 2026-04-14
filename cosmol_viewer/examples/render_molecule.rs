use cosmol_viewer::{Scene, Viewer, shapes::Molecule, utils::Stylable};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mol = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?.centered();

    let mut scene = Scene::new();

    scene.add_shape_with_id("mol", mol);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;

    let img = viewer.take_screenshot();

    img.save(Path::new("screenshot.png"))?;

    println!("Press Enter to exit...");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
