//! Run from the repository root:
//! cargo run -p cosmol_viewer --example highlight_residues
use cosmol_viewer::{Cartoon, ImageRenderer, Scene, Viewer, select};
use cosmolkit::Protein;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let protein = Protein::from_mmcif(include_str!("6fi1.cif"))?;

    // Zero-based internal residue indices, NOT the residue numbers in the CIF.
    // Change this range to choose which residues are highlighted.
    let residues = select(&protein).residues();
    let highlighted_ids = (10..15)
        .map(cosmolkit::BioResidueId::new)
        .collect::<Vec<_>>();
    let highlighted = residues.clone().filter_ids(&highlighted_ids);
    let remaining = residues.difference(&highlighted)?;

    let mut scene = Scene::new();
    // Both selections share complete-chain Cartoon geometry. Only the emitted
    // residue patches and colors differ, so selection boundaries remain joined.
    scene.set(
        "remaining",
        (&remaining, Cartoon::default().color("#AAB4C0")),
    )?;
    scene.set(
        "highlighted",
        (&highlighted, Cartoon::default().color("#FF4444")),
    )?;
    scene.recenter_on(&protein)?;
    scene.set_scale(0.2);
    scene.set_background_color("#021529");

    let output = concat!(env!("CARGO_MANIFEST_DIR"), "/highlight_residues.png");
    ImageRenderer::save_png(&scene, output, 1200, 900)?;
    println!("Highlighted {} residues; saved {output}", highlighted.len());

    let viewer = Viewer::show(&scene, 800.0, 500.0)?;
    viewer.keep_alive()?;
    Ok(())
}
