use cosmol_viewer::{BallAndStick, ImageRenderer, Scene, select};
use cosmolkit::Molecule;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mol = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?;

    let mut scene = Scene::new();
    scene.show_with_id(
        "mol",
        &mol,
        &select(&mol).atoms(),
        BallAndStick::default().outline("#050505", 0.04),
    )?;
    scene.recenter_on(&mol)?;

    let output = "offscreen_2400x1600.png";
    ImageRenderer::save_png(&scene, output, 2400, 1600)?;

    println!("saved {output}");
    Ok(())
}
