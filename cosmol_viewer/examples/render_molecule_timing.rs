use cosmol_viewer::{Scene, Viewer};
use cosmolkit::Molecule;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmol_viewer_core::register_render();

    let t_main = Instant::now();

    let mol = Molecule::from_sdf(include_str!("6fi1_ligand.sdf"))?;
    let mut scene = Scene::new();
    scene.set("mol", &mol)?;
    scene.recenter_on(&mol)?;

    let viewer = Viewer::show(&scene, 800.0, 500.0)?;
    let t_after_render = Instant::now();

    // Trigger one real scene change before the first update.
    scene.set_scale(1.02);
    viewer.update(&scene);
    let t_after_update = Instant::now();

    println!(
        "timing_ms main_to_render={:.3} main_to_first_update_end={:.3} render_to_first_update_end={:.3}",
        t_after_render.duration_since(t_main).as_secs_f64() * 1000.0,
        t_after_update.duration_since(t_main).as_secs_f64() * 1000.0,
        t_after_update.duration_since(t_after_render).as_secs_f64() * 1000.0
    );

    // Keep process lifecycle deterministic for benchmark-like use.
    std::process::exit(0);
}
