use cosmol_viewer::{BallAndStick, ImageRenderer, Scene, Viewer, cosmolkit};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let cosmolkit_mol =
        cosmolkit::Molecule::from_smiles("COc1cc(C=Nn2c(SC)nnc2c3ccccc3)c(Br)cc1O")?;
    // Coordinate generation is an explicit scientific operation owned by CK.
    let mol = cosmolkit_mol.with_hydrogens()?.with_2d_coordinates()?;

    let mut scene = Scene::new();
    scene.set_scale(0.8);
    scene.set(
        "mol",
        (&mol, BallAndStick::default().outline("#050505", 0.04)),
    )?;
    scene.recenter_on(&mol)?;

    ImageRenderer::save_png(&scene, "render_molecule_from_cosmolkit.png", 1200, 900)?;

    let viewer = Viewer::show(&scene, 800.0, 500.0)?;
    viewer.keep_alive()?;

    Ok(())
}
