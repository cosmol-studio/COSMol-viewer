use cosmol_viewer::shapes::Sphere;
use cosmol_viewer::utils::VisualShape;
use cosmol_viewer::{Scene, Viewer};
use std::{
    f32::consts::PI,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();

    let ids = ["a", "b", "c", "d", "e", "f"];
    for id in ids.iter() {
        let sphere = Sphere::new([0.0, 0.0, 0.0], 0.4).color([255, 255, 255]);
        scene.add_shape_with_id(*id, sphere);
    }

    scene.set_scale(2.0);

    let viewer = Viewer::render(&scene, 800.0, 500.0)?;

    // === time-driven animation ===
    let start_time = Instant::now();
    let angular_speed = 0.4 * PI;

    let frame_interval = Duration::from_millis(5);

    loop {
        let elapsed = start_time.elapsed().as_secs_f32();
        let t = elapsed * angular_speed;

        for (i, id) in ids.iter().enumerate() {
            let phase = i as f32 * PI / 3.0;
            let theta = t + phase;

            let x = 1.5 * f32::cos(theta);
            let y = 0.8 * f32::sin(theta);
            let z = 0.5 * f32::sin(theta * 2.0);

            let radius = 0.3 + 0.15 * f32::sin(theta * 1.5);

            let r = 0.5 + 0.5 * f32::sin(theta);
            let g = 0.5 + 0.5 * f32::cos(theta);
            let b = 1.0 - r;

            let sphere = Sphere::new([x, y, z], radius).color([
                (r * 256.0) as u8,
                (g * 256.0) as u8,
                (b * 256.0) as u8,
            ]);
            scene.replace_shape(id, sphere)?;
        }

        viewer.update(&scene);

        thread::sleep(frame_interval);
    }
}
