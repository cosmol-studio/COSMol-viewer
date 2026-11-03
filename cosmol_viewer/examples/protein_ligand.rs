use cosmol_viewer::{BallAndStick, Cartoon, ImageRenderer, Scene, Viewer};
use cosmolkit as ck;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let prot = ck::Protein::from_mmcif(include_str!("6fi1.cif"))?;
    let ligand = ck::Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?;

    let mut scene = Scene::new();
    scene.set("prot", (&prot, Cartoon::default().rainbow_residues()))?;
    scene.set(
        "ligand",
        (&ligand, BallAndStick::default().outline("#EEEEEE", 0.02)),
    )?;
    scene.recenter_on(&ligand)?;
    scene.set_background_color("#021529");

    ImageRenderer::save_png(&scene, "protein_ligand.png", 1200, 900)?;

    let viewer = Viewer::open(&scene, 800.0, 500.0)?;

    viewer.show_fps(true);
    viewer.show_camera_parameters(true);
    viewer.keep_alive()?;

    Ok(())
}
