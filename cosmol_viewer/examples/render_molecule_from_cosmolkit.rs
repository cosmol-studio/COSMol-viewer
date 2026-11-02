use cosmol_viewer::{BallAndStick, Scene, Viewer, cosmolkit, select};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let cosmolkit_mol =
        cosmolkit::Molecule::from_smiles("COc1cc(C=Nn2c(SC)nnc2c3ccccc3)c(Br)cc1O")?;
    // Coordinate generation is an explicit scientific operation owned by CK.
    let mol = cosmolkit_mol.with_hydrogens()?.with_2d_coordinates()?;

    let mut scene = Scene::new();
    scene.set_scale(0.8);
    scene.show_with_id(
        "mol",
        &mol,
        &select(&mol).atoms(),
        BallAndStick::default().outline("#050505", 0.04),
    )?;
    scene.recenter_on(&mol)?;

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;
    let screenshot = viewer.take_screenshot()?;
    screenshot.save(Path::new("screenshot.png"))?;

    viewer.keep_alive()?;

    Ok(())
}
