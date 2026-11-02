use cosmol_viewer::{Cartoon, RenderQuality, Scene, Surface, Viewer, select};
use cosmolkit::Protein;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let protein = Protein::from_mmcif(include_str!("6fi1.cif"))?;

    let mut scene = Scene::new();
    scene.set_scale(0.2);
    scene.show_with_id(
        "cartoon",
        &protein,
        &select(&protein).residues(),
        Cartoon::default().rainbow_residues(),
    )?;
    scene.show(
        &protein,
        &select(&protein).atoms(),
        Surface::default().color("#DCE8F2").opacity(0.9),
    )?;
    scene.recenter_on(&protein)?;
    scene.set_background_color("#021529");
    scene.set_depth_cue(true);

    let viewer = Viewer::render_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    viewer.keep_alive()?;

    Ok(())
}
