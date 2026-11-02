use cosmol_viewer::{Cartoon, RenderQuality, Scene, Viewer, select};
use cosmolkit::Protein;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let prot = Protein::from_mmcif(include_str!("6fi1.cif"))?;

    let mut scene = Scene::new();
    scene.set_scale(0.2);
    scene.show_with_id(
        "prot",
        &prot,
        &select(&prot).residues(),
        Cartoon::default().rainbow_residues(),
    )?;
    scene.recenter_on(&prot)?;
    scene.set_background_color("#021529");

    // Viewer::render(&scene, 800.0, 500.0)?;
    let viewer = Viewer::render_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    viewer.keep_alive()?;

    Ok(())
}
