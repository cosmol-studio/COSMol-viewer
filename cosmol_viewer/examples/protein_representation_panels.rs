//! Draw four representations of one protein segment and assemble them into a PNG.

use cosmol_viewer::{
    Cartoon, ImageRenderer, Multishadow, Scene, Silhouettes, Viewer, select,
    styles::{Sphere, Stick},
    utils::Color,
};
use cosmolkit::{BioResidueId, Element, Protein};
use image::{DynamicImage, Rgb, RgbImage, imageops};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let package = Path::new(env!("CARGO_MANIFEST_DIR"));
    let structure = package.join("../docs-web/data/showcase/1db1-receptor.pdb");
    let protein = Protein::read(&structure)?;
    let source = select(&protein);
    let (residues, atoms) = (source.residues(), source.atoms());

    // Zero-based CK IDs of one segment of the 1DB1 receptor.
    let residue_ids: Vec<_> = (151..174).map(BioResidueId::new).collect();
    let segment = residues.clone().filter_ids(&residue_ids);
    let heavy_atoms = atoms
        .clone()
        .residue_ids(&residue_ids)
        .difference(&atoms.clone().element(Element::H))?;
    let ca_ids: Vec<_> = protein
        .atoms()
        .filter(|atom| {
            residue_ids.contains(&atom.residue().id()) && atom.name().as_str().trim() == "CA"
        })
        .map(|atom| atom.id())
        .collect();
    let sparse_ca_ids: Vec<_> = ca_ids.iter().copied().step_by(2).collect();

    let mut scene = Scene::new();
    scene.recenter_on(&protein)?;
    scene.set_scale(0.222);
    scene.set_camera_view(-170.0, 4.0, 8.0, 35.0, [0.0, 0.8, 0.0], 18.0);
    scene.set_background_color("#FFFFFF");
    scene.set_lighting(0.55, 0.55, 0.14, 1.0, Color::from([255, 255, 255]).0);
    scene.set_multishadow(Some(Multishadow::default()))?;
    scene.set_silhouettes(Some(Silhouettes {
        color: Color::from([20, 20, 20]).0,
        width: 4.0,
        ..Silhouettes::default()
    }))?;
    let white = Cartoon::default().color("#FFFFFF").roughness(0.85);
    scene.set("context", (&protein, white.clone()))?;

    let blue = "#009ED6";
    let beads = Sphere::default().scale(0.62).color(blue);
    let (width, height, gap) = (680, 620, 16);
    let mut figure = RgbImage::from_pixel(4 * width + 3 * gap, height, Rgb([255; 3]));
    let mut draw_panel = |scene: &Scene, index: u32| -> Result<(), String> {
        let rendered = ImageRenderer::render(scene, width, height)?;
        let panel = DynamicImage::ImageRgba8(rendered).into_rgb8();
        imageops::replace(&mut figure, &panel, i64::from(index * (width + gap)), 0);
        Ok(())
    };

    scene.set(
        "detail",
        (atoms.clone().filter_ids(&sparse_ca_ids), beads.clone()),
    )?;
    draw_panel(&scene, 0)?;
    scene.set("detail", (atoms.filter_ids(&ca_ids), beads))?;
    draw_panel(&scene, 1)?;
    scene.set("detail", (heavy_atoms, Stick::default().color(blue)))?;
    draw_panel(&scene, 2)?;
    // Complementary selections reuse the same complete-chain Cartoon mesh.
    scene.set("context", (residues.difference(&segment)?, white))?;
    scene.set(
        "detail",
        (segment, Cartoon::default().color(blue).roughness(0.85)),
    )?;
    draw_panel(&scene, 3)?;

    let output = package.join("protein_representation_panels_rust.png");
    figure.save(&output)?;
    println!("Saved {}", output.display());

    Viewer::show(&scene, width as f32, height as f32)?.keep_alive()?;
    Ok(())
}
