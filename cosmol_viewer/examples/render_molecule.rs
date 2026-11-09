use cosmol_viewer::{BallAndStick, ImageRenderer, Scene, Viewer};
use cosmolkit::Molecule;

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

    ImageRenderer::save_png(&scene, "render_molecule.png", 2400, 1800)?;

    let viewer = Viewer::show(&scene, 800.0, 500.0)?;

    viewer.keep_alive()?;

    Ok(())
}
