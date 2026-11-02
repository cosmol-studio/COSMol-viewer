use cosmol_viewer::{Scene, Viewer, select, styles::Stick};
use cosmolkit::Molecule;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let mol = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?.with_kekulized_bonds()?;

    let mut scene = Scene::new();

    scene.show_with_id(
        "mol",
        &mol,
        &select(&mol).atoms(),
        Stick::default().outline("#050505", 0.04),
    )?;
    scene.recenter_on(&mol)?;
    scene.set_camera_view(35.0, 20.0, 0.0, 32.0, [0.0, 0.0, 0.0], 18.0);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;

    let img = viewer.take_screenshot()?;

    img.save(Path::new("screenshot_stick.png"))?;

    viewer.keep_alive()?;

    Ok(())
}
