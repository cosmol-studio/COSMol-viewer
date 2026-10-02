use cosmol_viewer::{Scene, Viewer, shapes::Molecule};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let mol = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?
        .centered()
        .enable_outline(0.04);

    let mut scene = Scene::new();

    scene.add_shape_with_id("mol", mol);
    scene.set_auto_rotate(true, 20.0);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;

    let img = viewer.take_screenshot()?;

    println!("screenshot saved to screenshot.png");

    img.save(Path::new("screenshot.png"))?;

    viewer.keep_alive()?;

    Ok(())
}
