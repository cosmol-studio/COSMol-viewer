//! Run from the repository root:
//! cargo run -p cosmol_viewer --example molecule_colors
//!
//! Compare the scene palette, two displays that reuse atom colors, and a
//! separate scheme in the interactive window. Both PNGs are exported
//! offscreen into cosmol_viewer/.
use cosmol_viewer::{ImageRenderer, MoleculeColors, Scene, Viewer, styles::Stick};
use cosmolkit::Molecule;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let ligand = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?.with_kekulized_bonds()?;
    let xyz = ligand.coordinates_3d(0)?;
    let center: [f64; 3] =
        std::array::from_fn(|axis| xyz.iter().map(|p| p[axis]).sum::<f64>() / xyz.len() as f64);
    let place = |x: f64, y: f64| {
        // Arrange copies in a grid so their colors can be compared side by side.
        let coordinates = xyz
            .iter()
            .map(|p| [p[2] - center[2] + x, p[1] - center[1] + y, center[0] - p[0]])
            .collect();
        ligand.with_3d_coordinates(coordinates)
    };
    let reference = place(-5.5, 4.8)?;
    let first = place(5.5, 4.8)?;
    let second = place(-5.5, -4.8)?;
    let different = place(5.5, -4.8)?;

    let mut scene = Scene::new();
    scene.set_background_color("#021529");
    scene.set_camera_view(0.0, 0.0, 0.0, 39.0, [0.0; 3], 32.0);

    // Top left uses the scene's carbon color. Other elements stay built-in.
    let palette = MoleculeColors::new([("C", [92, 203, 229].into())])?;
    scene.set_molecule_palette(&palette)?;
    let sticks = Stick::default();
    scene.set("palette", (&reference, sticks))?;

    // Top right and bottom left share one scheme. IDs are zero-based CK atom IDs.
    let shared =
        palette.with_atom_colors([(0, [255, 209, 102].into()), (3, [255, 209, 102].into())])?;
    let highlighted = sticks.colors(&shared);
    scene.set("shared-a", (&first, highlighted.clone()))?;
    scene.set("shared-b", (&second, highlighted))?;

    // Bottom right uses a different color for the same atom IDs.
    let pink =
        palette.with_atom_colors([(0, [242, 131, 203].into()), (3, [242, 131, 203].into())])?;
    scene.set("different", (&different, sticks.colors(&pink)))?;

    let custom_png = concat!(env!("CARGO_MANIFEST_DIR"), "/molecule_colors.png");
    ImageRenderer::save_png(&scene, custom_png, 1200, 900)?;

    // Reset the palette and replace existing displays to reproject their colors.
    let mut restored = scene.clone();
    restored.set_molecule_palette(&MoleculeColors::default())?;
    for (name, molecule) in [
        ("palette", &reference),
        ("shared-a", &first),
        ("shared-b", &second),
        ("different", &different),
    ] {
        restored.set(name, (molecule, sticks))?;
    }
    let restored_png = concat!(env!("CARGO_MANIFEST_DIR"), "/molecule_colors_builtin.png");
    ImageRenderer::save_png(&restored, restored_png, 1200, 900)?;
    println!("Custom colors: {custom_png}");
    println!("Built-in colors: {restored_png}");

    let viewer = Viewer::show(&scene, 800.0, 600.0)?;
    viewer.keep_alive()?;
    Ok(())
}
