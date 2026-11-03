// Interactive window and framebuffer screenshot example.
use cosmol_viewer::{BallAndStick, Scene, Viewer};
use cosmolkit::Molecule;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let mol = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?;

    let mut scene = Scene::new();

    scene.set(
        "mol",
        (&mol, BallAndStick::default().outline("#050505", 0.04)),
    )?;
    scene.recenter_on(&mol)?;
    scene.set_auto_rotate(true, 20.0);

    let viewer = Viewer::open(&scene, 800.0, 500.0)?;

    let img = viewer.take_screenshot()?;

    println!("screenshot saved to viewer_screenshot.png");

    img.save(Path::new("viewer_screenshot.png"))?;

    viewer.keep_alive()?;

    Ok(())
}
