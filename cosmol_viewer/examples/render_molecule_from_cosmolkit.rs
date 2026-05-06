use cosmol_viewer::{Scene, Viewer, cosmolkit, shapes::Molecule, utils::Stylable};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cosmolkit_mol = cosmolkit::Molecule::from_smiles("c1ccccc1")?;
    let mol = Molecule::from_cosmolkit(&cosmolkit_mol)?
        .centered()
        .color("#5B8DEF");

    let mut scene = Scene::new();
    scene.enable_outline();
    scene.outline.width = 0.04;
    scene.add_shape_with_id("mol", mol);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;
    viewer.take_screenshot().save(Path::new("screenshot.png"))?;

    println!("Press Enter to exit...");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
