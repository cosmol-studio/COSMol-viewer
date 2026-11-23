//! Render the 1IEP binding site with flat colors, silhouettes, and depth cueing.

use cosmol_viewer::{
    Cartoon, ImageRenderer, MoleculeColors, Scene, Viewer, styles::Stick, utils::Color,
};
use cosmolkit::{Protein, SdfRecordStream};
use std::{fs::File, io::BufReader, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs-web/data/showcase");
    let protein = Protein::read(&assets.join("1iep-receptor.pdb"))?;
    // The first SDF record is the crystallographic ligand.
    let poses = BufReader::new(File::open(assets.join("imatinib-poses.sdf"))?);
    let crystal = SdfRecordStream::new(poses)
        .next()
        .ok_or("Missing the 1IEP crystal ligand")??;
    let ligand = crystal.molecule()?;
    let green = Color::from([165, 255, 161]);
    let colors = MoleculeColors::new([("C", green), ("N", [67, 93, 242].into()), ("O", green)])?;
    let outline = "#404040";

    let mut scene = Scene::new();
    let ribbon = Cartoon::default()
        .color("#74ACFF")
        .silhouettes(outline, 4.0);
    scene.set("receptor", (&protein, ribbon))?;
    let sticks = Stick::default()
        .show_bond_order(false)
        .scale(1.45)
        .silhouettes(outline, 4.0)
        .colors(&colors);
    scene.set("ligand", (ligand, sticks))?;
    scene.recenter_on(ligand)?;
    scene.set_scale(0.8);
    scene.set_camera_view(77.0, -22.0, 53.0, 34.0, [0.0; 3], 22.0);
    scene.set_background_color("#FFFFFF");
    scene.set_lighting(1.0, 0.0, 0.0, 1.0, Color::from([255; 3]).0);
    scene.set_depth_cue(true);

    ImageRenderer::save_png(&scene, "flat_binding_site.png", 1400, 840)?;
    Viewer::show(&scene, 1000.0, 600.0)?.keep_alive()?;
    Ok(())
}
