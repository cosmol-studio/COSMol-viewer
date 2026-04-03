use cosmol_viewer::shapes::Sphere;
use cosmol_viewer::utils::Stylable;
use cosmol_viewer::{Animation, Scene, Viewer};
use std::f32::consts::PI;

fn main() {
    // Sphere ID
    let ids = ["a", "b", "c", "d", "e", "f"];

    // Animation parameters
    let interval: f32 = 0.05; // Time interval between frames (seconds)
    let duration: f32 = 10.0; // Total animation duration (seconds)
    let num_frames = (duration / interval) as usize;

    // Store all frames
    let mut animation = Animation::new(interval, -1, true);

    for frame_idx in 0..num_frames {
        let t = frame_idx as f32 * interval / duration * PI * 4.0;

        let mut scene = Scene::new();
        scene.set_scale(2.0);

        for (i, id) in ids.iter().enumerate() {
            let phase = i as f32 * PI / 3.0;
            let theta = t + phase;

            // Trajectory: elliptical motion
            let x = 1.5 * f32::cos(theta);
            let y = 0.8 * f32::sin(theta);
            let z = 0.5 * f32::sin(theta * 1.5);

            // Radius: pulsating change
            let radius = 0.3 + 0.15 * f32::sin(theta * 1.5);

            // Color: dynamic RGB gradient
            let r = 0.5 + 0.5 * f32::sin(theta);
            let g = 0.5 + 0.5 * f32::cos(theta);
            let b = 1.0 - r;

            let sphere = Sphere::new([x, y, z], radius).color([
                (r * 256.0) as u8,
                (g * 256.0) as u8,
                (b * 256.0) as u8,
            ]);

            scene.add_shape_with_id(*id, sphere);
        }

        animation.add_frame(scene);
    }

    // Submit all frames at once; Viewer controls playback
    let _ = Viewer::play(animation, 800.0, 500.0);
}
