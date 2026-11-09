use cosmol_viewer::{Cartoon, ImageRenderer, RenderQuality, Scene, Surface, Viewer};
use cosmolkit::Protein;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let protein = Protein::from_mmcif(include_str!("6fi1.cif"))?;

    let mut scene = Scene::new();
    scene.set_scale(0.2);
    scene.set("cartoon", (&protein, Cartoon::default().rainbow_residues()))?;
    scene.add((&protein, Surface::default().color("#DCE8F2").opacity(0.9)))?;
    scene.recenter_on(&protein)?;
    scene.set_background_color("#021529");
    scene.set_depth_cue(true);

    ImageRenderer::save_png(&scene, "render_protein_surface.png", 1200, 900)?;

    let viewer = Viewer::show_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    viewer.keep_alive()?;

    Ok(())
}
