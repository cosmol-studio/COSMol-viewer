use cosmol_viewer::{RenderQuality, Scene, Viewer, shapes::Protein};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let protein = Protein::from_mmcif(include_str!("./6fi1.cif"))?
        .centered()
        .rainbow_residues();

    let protein_surface = Protein::from_mmcif(include_str!("6fi1.cif"))?
        .centered()
        .surface()
        .color("#DCE8F2")
        .opacity(0.9);

    let mut scene = Scene::new();
    scene.set_scale(0.2);
    scene.add_shape(protein);
    scene.add_shape(protein_surface);
    scene.set_background_color("#021529");
    scene.set_depth_cue(true);

    let viewer = Viewer::render_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    viewer.keep_alive()?;

    Ok(())
}
