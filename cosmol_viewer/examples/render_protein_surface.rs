use cosmol_viewer::{RenderQuality, Scene, Viewer, shapes::Protein};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let _viewer = Viewer::render_with_quality(&scene, 800.0, 500.0, RenderQuality::High)?;

    println!("Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(())
}
