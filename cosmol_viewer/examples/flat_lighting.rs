//! Show flat element colors with depth-based silhouettes.

use cosmol_viewer::{
    BallAndStick, ImageRenderer, MoleculeColors, Scene, Silhouettes, Viewer, utils::Color,
};
use cosmolkit::Molecule;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let ligand = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?.with_kekulized_bonds()?;
    let colors = MoleculeColors::new([("C", [255, 197, 50].into())])?;

    let mut scene = Scene::new();
    scene.set("ligand", (&ligand, BallAndStick::default().colors(&colors)))?;
    scene.recenter_on(&ligand)?;
    scene.set_camera_view(-55.0, 20.0, 0.0, 22.0, [0.0; 3], 22.0);
    scene.set_background_color("#FFFFFF");
    // Constant white ambient light preserves the element colors.
    scene.set_lighting(1.0, 0.0, 0.0, 1.0, Color::from([255; 3]).0);
    scene.set_silhouettes(Some(Silhouettes {
        color: Color::from([64; 3]).0,
        width: 4.0,
        depth_jump: 0.03,
    }))?;

    ImageRenderer::save_png(&scene, "flat_lighting.png", 1200, 900)?;
    Viewer::show(&scene, 800.0, 600.0)?.keep_alive()?;
    Ok(())
}
