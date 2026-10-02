use cosmol_viewer::{Scene, Viewer, shapes::Molecule};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let mol = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?
        .centered()
        .stick()
        .enable_outline(0.04);

    let mut scene = Scene::new();

    scene.add_shape_with_id("mol", mol);
    scene.set_camera_view(35.0, 20.0, 0.0, 32.0, [0.0, 0.0, 0.0], 18.0);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;

    let img = viewer.take_screenshot()?;

    img.save(Path::new("screenshot_stick.png"))?;

    viewer.keep_alive()?;

    Ok(())
}
