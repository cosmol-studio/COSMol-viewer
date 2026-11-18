use cosmol_viewer::styles::Stick;
use cosmol_viewer::{ImageRenderer, Scene, Viewer, select};
use cosmolkit as ck;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();
    let protein = ck::Protein::from_mmcif(include_str!("6fi1.cif"))?;
    let atoms = select(&protein).atoms();
    let ids = (10..15).map(ck::BioResidueId::new).collect::<Vec<_>>();
    let highlighted = atoms.clone().residue_ids(&ids).sidechain();
    let rest = atoms.difference(&highlighted)?;
    let mut scene = Scene::new();
    scene.set("protein", (&rest, Stick::default()))?;
    scene.set(
        "sidechains",
        (&highlighted, Stick::default().color("#FF4444")),
    )?;
    scene.recenter_on(&protein)?;
    scene.set_scale(0.2);
    scene.set_background_color("#021529");
    ImageRenderer::save_png(
        &scene,
        concat!(env!("CARGO_MANIFEST_DIR"), "/protein_sticks.png"),
        1200,
        900,
    )?;
    let viewer = Viewer::show(&scene, 800.0, 600.0)?;
    viewer.keep_alive()?;
    Ok(())
}
