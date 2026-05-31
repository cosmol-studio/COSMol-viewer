use cosmol_viewer::{Scene, Viewer, cosmolkit, shapes::Molecule};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cosmolkit_mol =
        cosmolkit::Molecule::from_smiles("COc1cc(C=Nn2c(SC)nnc2c3ccccc3)c(Br)cc1O")?;
    let cosmolkit_mol = cosmolkit_mol.with_hydrogens()?;
    let mol = Molecule::from_cosmolkit(&cosmolkit_mol)?
        .centered()
        .enable_outline(0.04);

    let mut scene = Scene::new();
    scene.set_scale(0.8);
    scene.add_shape_with_id("mol", mol);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;
    viewer.take_screenshot().save(Path::new("screenshot.png"))?;

    println!("Press Enter to exit...");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
