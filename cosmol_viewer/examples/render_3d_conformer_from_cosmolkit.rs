use cosmol_viewer::{Scene, Viewer, cosmolkit, shapes::Molecule};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = cosmolkit::Molecule::from_smiles("Oc1ccccc1-c2ccccc2O")?.sanitize()?;

    let mut params = cosmolkit::EmbedParameters::etkdg_v3();
    params.random_seed = 0xF00D;
    params.num_threads = 1;

    let cosmolkit_mol = base.with_3d_conformer_with_params(params)?;
    let mol = Molecule::from_cosmolkit(&cosmolkit_mol)?
        .centered()
        .enable_outline(0.04);

    let mut scene = Scene::new();
    scene.set_scale(0.9);
    scene.set_camera_view(35.0, 22.0, 0.0, 28.0, [0.0, 0.0, 0.0], 18.0);
    scene.add_shape_with_id("conformer", mol);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;
    viewer
        .take_screenshot()
        .save(Path::new("cosmolkit_3d_conformer_viewer.png"))?;

    println!("Press Enter to exit...");
    use std::io::{self, Write};
    let _ = io::stdout().flush();
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}
