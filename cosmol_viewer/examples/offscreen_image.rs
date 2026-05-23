use cosmol_viewer::{ImageRenderer, Scene, shapes::Molecule};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mol = Molecule::from_sdf(include_str!("../examples/6fi1_ligand.sdf"))?
        .centered()
        .enable_outline(0.04);

    let mut scene = Scene::new();
    scene.add_shape_with_id("mol", mol);

    let output = "offscreen_2400x1600.png";
    ImageRenderer::save_png(&scene, output, 2400, 1600)?;

    println!("saved {output}");
    Ok(())
}
