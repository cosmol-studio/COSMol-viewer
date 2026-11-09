use cosmol_viewer::{ImageRenderer, Scene, Viewer, styles::Stick};
use cosmolkit::Molecule;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let mol = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?.with_kekulized_bonds()?;

    let mut scene = Scene::new();

    scene.set("mol", (&mol, Stick::default().outline("#050505", 0.04)))?;
    scene.recenter_on(&mol)?;
    scene.set_camera_view(35.0, 20.0, 0.0, 32.0, [0.0, 0.0, 0.0], 18.0);

    ImageRenderer::save_png(&scene, "render_molecule_stick.png", 1200, 900)?;

    let viewer = Viewer::show(&scene, 800.0, 500.0)?;

    viewer.keep_alive()?;

    Ok(())
}
