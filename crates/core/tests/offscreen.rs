use cosmol_viewer_core::{
    ImageBackground, ImageRenderer, scene::Scene, selection::select, shapes::Sphere,
    styles::BallAndStick,
};

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
    let molecule = cosmolkit::Molecule::from_sdf(include_str!(
        "../../../cosmol_viewer/examples/6fi1_ligand.sdf"
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene
        .show_with_id(
            "molecule",
            &molecule,
            &select(&molecule).atoms(),
            BallAndStick::default().outline("#050505", 0.04),
        )
        .unwrap();
    scene.recenter_on(&molecule).unwrap();
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

fn translucent_scene() -> Scene {
    let mut scene = Scene::new();
    scene.set_depth_cue(false);
    scene.add_shape(
        Sphere::new([0.0, 0.0, 0.0], 0.9)
            .color("#AABBE3")
            .opacity(0.45),
    );
    scene.add_shape(Sphere::new([0.3, 0.0, -0.65], 0.55).color("#7292C7"));
    scene
}

#[test]
#[ignore = "requires an available OpenGL backend"]
fn transparent_export_composites_like_direct_opaque_rendering() {
    let scene = translucent_scene();
    let transparent = ImageRenderer::render_with_background(
        &scene,
        193,
        127,
        ImageBackground::Color([0.0, 0.0, 0.0, 0.0]),
    )
    .unwrap();
    assert!(
        transparent
            .pixels()
            .any(|pixel| pixel[3] > 0 && pixel[3] < 255)
    );
    assert!(transparent.pixels().any(|pixel| pixel[3] == 255));
    for background in [
        [1.0, 1.0, 1.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
        [0.2, 0.6, 0.8, 1.0],
    ] {
        let direct = ImageRenderer::render_with_background(
            &scene,
            193,
            127,
            ImageBackground::Color(background),
        )
        .unwrap();
        let mut max_error = 0.0_f32;
        let mut total_error = 0.0;
        for (transparent, opaque) in transparent.pixels().zip(direct.pixels()) {
            assert_eq!(opaque[3], 255);
            let alpha = f32::from(transparent[3]) / 255.0;
            for channel in 0..3 {
                let composited = f32::from(transparent[channel]) * alpha
                    + background[channel] * 255.0 * (1.0 - alpha);
                let error = (composited - f32::from(opaque[channel])).abs();
                max_error = max_error.max(error);
                total_error += error;
            }
        }
        let mean_error = total_error / (193.0 * 127.0 * 3.0);
        assert!(
            max_error <= 3.0 && mean_error < 0.5,
            "background={background:?}: max={max_error}, mean={mean_error}"
        );
    }
    let bytes = ImageRenderer::render_png_bytes_with_background(
        &scene,
        193,
        127,
        ImageBackground::Color([0.0, 0.0, 0.0, 0.0]),
    )
    .unwrap();
    assert_eq!(
        image::load_from_memory(&bytes).unwrap().to_rgba8(),
        transparent
    );

    // Scene-selected transparency must use the same output conversion.
    let mut scene = scene;
    scene.set_transparent_background(true);
    assert_eq!(
        ImageRenderer::render(&scene, 193, 127).unwrap(),
        transparent
    );
}

#[test]
#[ignore = "requires an available OpenGL backend"]
fn translucent_colored_background_is_premultiplied_before_blending() {
    let scene = translucent_scene();
    let background = [0.8, 0.4, 0.2, 0.5];
    let transparent = ImageRenderer::render_with_background(
        &scene,
        193,
        127,
        ImageBackground::Color([0.0, 0.0, 0.0, 0.0]),
    )
    .unwrap();
    let colored =
        ImageRenderer::render_with_background(&scene, 193, 127, ImageBackground::Color(background))
            .unwrap();
    for (foreground, result) in transparent.pixels().zip(colored.pixels()) {
        let foreground_alpha = f32::from(foreground[3]) / 255.0;
        let result_alpha = f32::from(result[3]) / 255.0;
        let expected_alpha = foreground_alpha + background[3] * (1.0 - foreground_alpha);
        assert!((result_alpha - expected_alpha).abs() <= 2.0 / 255.0);
        for channel in 0..3 {
            let expected_premultiplied = f32::from(foreground[channel]) * foreground_alpha
                + background[channel] * 255.0 * background[3] * (1.0 - foreground_alpha);
            let actual_premultiplied = f32::from(result[channel]) * result_alpha;
            assert!(
                (actual_premultiplied - expected_premultiplied).abs() <= 3.0,
                "foreground={foreground:?}, result={result:?}, channel={channel}"
            );
        }
    }
    // The caller's straight-alpha background color is recovered at empty pixels.
    for (actual, expected) in colored
        .get_pixel(0, 0)
        .0
        .into_iter()
        .zip([204_i16, 102, 51, 128])
    {
        assert!((i16::from(actual) - expected).abs() <= 2);
    }
}

#[test]
#[ignore = "requires an available OpenGL backend"]
fn renders_ck_protein_ligand_after_geometry_only_transport() {
    let protein =
        cosmolkit::Protein::from_mmcif(include_str!("../../../cosmol_viewer/examples/6fi1.cif"))
            .unwrap();
    let ligand = cosmolkit::Molecule::from_sdf(include_str!(
        "../../../cosmol_viewer/examples/6fi1_ligand.sdf"
    ))
    .unwrap();
    let mut scene = Scene::new();
    scene
        .show(
            &protein,
            &select(&protein).residues(),
            cosmol_viewer_core::styles::Cartoon::default().rainbow_residues(),
        )
        .unwrap();
    scene
        .show_with_id(
            "ligand",
            &ligand,
            &select(&ligand).atoms(),
            BallAndStick::default().outline("#EEEEEE", 0.02),
        )
        .unwrap();
    scene.recenter_on(&ligand).unwrap();
    let wire = postcard::to_allocvec(&scene).unwrap();
    let received: Scene = postcard::from_bytes(&wire).unwrap();
    let image = ImageRenderer::render_with_background(
        &received,
        193,
        127,
        ImageBackground::Color([0.0, 1.0, 0.0, 1.0]),
    )
    .unwrap();
    assert!(image.pixels().filter(|p| p.0 != [0, 255, 0, 255]).count() > 100);
    assert!(image.pixels().all(|p| p.0[3] == 255));
}
