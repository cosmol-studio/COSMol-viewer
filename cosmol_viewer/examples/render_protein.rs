use cosmol_viewer::{Cartoon, ImageRenderer, RenderQuality, Scene, Viewer};
use cosmolkit::Protein;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let prot = Protein::from_mmcif(include_str!("6fi1.cif"))?;

    let mut scene = Scene::new();
    scene.set_scale(0.2);
    scene.set("prot", (&prot, Cartoon::default().rainbow_residues()))?;
    scene.recenter_on(&prot)?;
    scene.set_background_color("#021529");

    ImageRenderer::save_png(&scene, "render_protein.png", 1200, 900)?;

    let viewer = Viewer::show_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    viewer.keep_alive()?;

    Ok(())
}
