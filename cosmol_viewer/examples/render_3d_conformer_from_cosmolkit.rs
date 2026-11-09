use cosmol_viewer::{BallAndStick, ImageRenderer, Scene, Viewer, cosmolkit};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let base = cosmolkit::Molecule::from_smiles("CC(=O)Nc1ccc(O)cc1")?.sanitize()?;

    let mut params = cosmolkit::EmbedParams::etkdg_v3();
    params.random_seed = 0xF00D;
    params.num_threads = 1;

    let mol = base.with_3d_conformer_with_params(&params)?;

    let mut scene = Scene::new();
    scene.set_scale(0.9);
    scene.set_camera_view(35.0, 22.0, 0.0, 28.0, [0.0, 0.0, 0.0], 18.0);
    scene.set(
        "conformer",
        (&mol, BallAndStick::default().outline("#050505", 0.04)),
    )?;
    scene.recenter_on(&mol)?;

    ImageRenderer::save_png(&scene, "render_3d_conformer_from_cosmolkit.png", 1200, 900)?;

    let viewer = Viewer::show(&scene, 800.0, 500.0)?;
    viewer.keep_alive()?;

    Ok(())
}
