use cosmol_viewer_core::{ImageBackground, ImageRenderer, scene::Scene, shapes::Molecule};

#[test]
fn rejects_zero_dimensions_before_initializing_gl() {
    let scene = Scene::new();
    for (width, height) in [(0, 32), (32, 0), (0, 0)] {
        let err = ImageRenderer::render(&scene, width, height).unwrap_err();
        assert_eq!(err, "width and height must be non-zero");
    }
}

// Requires an OpenGL-capable device/driver. Run explicitly with:
// cargo test -p cosmol_viewer_core --test offscreen -- --ignored --nocapture
#[test]
#[ignore = "requires an available OpenGL backend"]
fn renders_repeatedly_with_background_alpha_and_png_encoding() {
    let molecule = Molecule::from_sdf(include_str!(
        "../../../cosmol_viewer/examples/6fi1_ligand.sdf"
    ))
    .unwrap()
    .centered()
    .enable_outline(0.04);
    let mut scene = Scene::new();
    scene.add_shape_with_id("molecule", molecule);
    let original_scene = serde_json::to_value(&scene).unwrap();

    // Recreate and destroy contexts on the same thread, with varying image sizes.
    for frame in 0..6 {
        let width = 193 + frame * 2;
        let height = 127 + frame * 2;
        let image = ImageRenderer::render_with_background(
            &scene,
            width,
            height,
            ImageBackground::Color([0.0, 1.0, 0.0, 1.0]),
        )
        .unwrap();
        assert_eq!(image.dimensions(), (width, height));
        assert_eq!(image.get_pixel(0, 0).0, [0, 255, 0, 255]);
        assert!(image.pixels().filter(|p| p.0 != [0, 255, 0, 255]).count() > 20);
        assert!(image.pixels().all(|p| p.0[3] == 255));
    }

    let transparent = ImageRenderer::render_with_background(
        &scene,
        193,
        127,
        ImageBackground::Color([0.0, 0.0, 0.0, 0.0]),
    )
    .unwrap();
    assert_eq!(transparent.get_pixel(0, 0).0, [0, 0, 0, 0]);
    assert!(transparent.pixels().any(|p| p.0[3] > 0));

    let png = ImageRenderer::render_png_bytes(&scene, 193, 127).unwrap();
    let decoded = image::load_from_memory(&png).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (193, 127));
    assert_eq!(serde_json::to_value(&scene).unwrap(), original_scene);
}
