use cosmol_viewer_wasm::{Animation, Scene};
use serde_json::{Value, from_str};

fn state(scene: &Scene) -> Value {
    from_str(&scene.to_json().unwrap()).unwrap()
}

#[test]
fn scene_handles_share_state_but_explicit_copies_are_independent() {
    let scene = Scene::new();
    scene.add_sphere("atom".into(), vec![0.0; 3], 1.0).unwrap();
    let handle = scene.clone();
    let copy = scene.clone_scene();
    handle.set_scale(2.0).unwrap();
    assert_eq!(state(&scene)["scale"], 2.0);
    assert_eq!(state(&copy)["scale"], 1.0);
    scene.merge_shapes(&scene);
    // Core merge preserves both shapes, assigning a fresh id on collision.
    assert_eq!(scene.shape_count(), 2);
    handle.remove_shape("atom".into()).unwrap();
    assert_eq!(scene.shape_count(), 1);
    assert_eq!(copy.shape_count(), 1);
}

#[test]
fn animation_borrows_reusable_scenes_and_captures_snapshots() {
    let scene = Scene::new();
    let animation = Animation::new(0.1, -1, true);
    animation.add_frame(&scene);
    scene.set_scale(2.0).unwrap();
    animation.add_frame(&scene);
    animation.set_static_scene(&scene);
    scene.set_scale(3.0).unwrap();
    let animation_state: Value = from_str(&animation.to_json().unwrap()).unwrap();
    assert_eq!(animation.frame_count(), 2);
    assert_eq!(animation_state["frames"][0]["scale"], 1.0);
    assert_eq!(animation_state["frames"][1]["scale"], 2.0);
    assert_eq!(animation_state["static_scene"]["scale"], 2.0);
    assert_eq!(state(&scene)["scale"], 3.0);
}

#[test]
fn invalid_geometry_and_vectors_leave_the_scene_usable() {
    let scene = Scene::new();
    assert!(scene.add_sphere("bad".into(), vec![0.0; 2], 1.0).is_err());
    assert!(scene.add_sphere("bad".into(), vec![f32::NAN; 3], 1.0).is_err());
    assert!(scene.add_sphere("bad".into(), vec![0.0; 3], -1.0).is_err());
    assert!(scene.set_background_color(vec![255, 0]).is_err());
    assert!(scene.set_scale(0.0).is_err());
    assert!(scene.set_depth_cue_range(0.8, 0.2).is_err());
    assert_eq!(scene.shape_count(), 0);
    scene.add_stick("bond".into(), vec![0.0; 3], vec![1.0; 3], 0.2).unwrap();
    assert_eq!(scene.shape_count(), 1);
}
