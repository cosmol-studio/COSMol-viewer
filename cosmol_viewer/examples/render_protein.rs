use cosmol_viewer::{RenderQuality, Scene, Viewer, shapes::Protein};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let prot = Protein::from_mmcif(include_str!("../examples/6fi1.cif"))?.rainbow_residues();

    let mut scene = Scene::new();
    scene.set_scale(0.2);
    scene.recenter(prot.get_center());
    scene.add_shape_with_id("prot", prot);
    scene.set_background_color("#021529");

    // Viewer::render(&scene, 800.0, 500.0)?;
    let viewer = Viewer::render_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    viewer.keep_alive()?;

    Ok(())
}
