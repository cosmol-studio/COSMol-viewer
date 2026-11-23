//! Run from the repository root with `cargo run -p cosmol_viewer --example scene_silhouettes`.

use cosmol_viewer::{
    Cartoon, ImageRenderer, Scene, Silhouettes, Surface, Viewer, select, styles::Stick,
    utils::Color,
};
use cosmolkit::{Molecule, Protein};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let protein = Protein::from_mmcif(include_str!("6fi1.cif"))?;
    let ligand = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?.with_kekulized_bonds()?;

    let mut scene = Scene::new();
    scene.set_background_color("#FFFFFF");
    scene.set_lighting(0.55, 0.55, 0.14, 1.0, Color::from([255, 255, 255]).0);
    scene.recenter_on(&protein)?;
    scene.set_scale(0.18);
    scene.set_camera_view(-90.0, -20.0, 0.0, 32.0, [0.0; 3], 15.0);
    scene.set(
        "protein",
        (
            &protein,
            Cartoon::default().color("#FFFFFF").roughness(0.85),
        ),
    )?;
    scene.set("ligand", (&ligand, Stick::default()))?;

    let output = Path::new(env!("CARGO_MANIFEST_DIR"));
    let save = |scene: &Scene, name| ImageRenderer::save_png(scene, output.join(name), 1280, 800);
    save(&scene, "scene_silhouettes_off.png")?;

    // A single scene setting draws edges for both the ribbon and the ligand.
    scene.set_silhouettes(Some(Silhouettes {
        color: Color::from([80, 80, 80]).0,
        width: 4.0,
        ..Silhouettes::default()
    }))?;
    save(&scene, "scene_silhouettes_on.png")?;

    // The ribbon and ligand outlines remain visible through the surface.
    let shell = Surface::default().color("#A9CBE2").opacity(0.30);
    scene.set("surface", (select(&protein).atoms(), shell))?;
    save(&scene, "scene_silhouettes_surface.png")?;
    println!("Saved three scene_silhouettes PNGs in {}", output.display());

    Viewer::show(&scene, 800.0, 500.0)?.keep_alive()?;
    Ok(())
}
