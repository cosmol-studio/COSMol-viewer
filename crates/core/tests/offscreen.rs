use cosmol_viewer_core::{
    ImageBackground, ImageRenderer,
    scene::{Multishadow, Scene, Silhouettes},
    selection::select,
    shapes::{Sphere, Stick},
    styles::{BallAndStick, Cartoon, Surface},
    utils::Color,
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
        .set(
            "molecule",
            (
                &select(&molecule).atoms(),
                BallAndStick::default().silhouettes("#050505", 2.0),
            ),
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
    scene
        .add(
            Sphere::new([0.0, 0.0, 0.0], 0.9)
                .color("#AABBE3")
                .opacity(0.45),
        )
        .unwrap();
    scene
        .add(Sphere::new([0.3, 0.0, -0.65], 0.55).color("#7292C7"))
        .unwrap();
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
        .add((
            &select(&protein).residues(),
            cosmol_viewer_core::styles::Cartoon::default().rainbow_residues(),
        ))
        .unwrap();
    scene
        .set(
            "ligand",
            (
                &select(&ligand).atoms(),
                BallAndStick::default().silhouettes("#EEEEEE", 1.0),
            ),
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

#[test]
#[ignore = "requires an available OpenGL backend"]
fn silhouettes_follow_visible_geometry_across_all_draw_passes() {
    let protein =
        cosmolkit::Protein::from_mmcif(include_str!("../../../cosmol_viewer/examples/6fi1.cif"))
            .unwrap();
    let selector = select(&protein);
    let ink = [220, 17, 155];
    let settings = Silhouettes {
        color: Color::from(ink).0,
        ..Silhouettes::default()
    };
    let count_edges = |scene: &Scene| {
        let image = ImageRenderer::render(scene, 240, 200).unwrap();
        image
            .pixels()
            .filter(|pixel| pixel.0 == [ink[0], ink[1], ink[2], 255])
            .count()
    };

    let mut primitives = Scene::new();
    primitives.set_silhouettes(Some(settings)).unwrap();
    primitives.add(Sphere::new([-1.6, 0.0, 0.0], 1.0)).unwrap();
    let sphere_edges = count_edges(&primitives);
    assert!(sphere_edges > 32);
    primitives
        .add(Stick::new([0.5, -1.2, 0.0], [2.5, 1.2, 0.0], 0.28))
        .unwrap();
    assert!(count_edges(&primitives) > sphere_edges + 32);

    let mut molecules = Scene::new();
    molecules.recenter_on(&protein).unwrap();
    molecules.set_scale(0.2);
    molecules
        .add((&protein, Cartoon::default().color("#B3B3B3")))
        .unwrap();
    assert_eq!(count_edges(&molecules), 0);
    molecules.set_silhouettes(Some(settings)).unwrap();
    assert!(count_edges(&molecules) > 300);
    molecules
        .add((&selector.atoms(), Surface::default().opacity(0.45)))
        .unwrap();
    assert!(count_edges(&molecules) > 100);
    molecules.set_silhouettes(None).unwrap();
    assert_eq!(count_edges(&molecules), 0);
}

#[test]
#[ignore = "requires an available OpenGL backend"]
fn shape_silhouettes_override_the_scene_default_and_respect_occlusion() {
    let red = [220, 17, 155];
    let green = [25, 161, 79];
    let left = Sphere::new([-1.6, 0.0, 0.0], 1.0);
    let mut scene = Scene::new();
    scene.set_camera_view(0.0, 0.0, 0.0, 8.0, [0.0; 3], 38.0);
    scene.set("left", left.silhouettes(red, 3.0)).unwrap();
    scene
        .set("right", Sphere::new([1.6, 0.0, 0.0], 1.0))
        .unwrap();
    let count = |image: &image::RgbaImage, color: [u8; 3], x: std::ops::Range<u32>| {
        image
            .enumerate_pixels()
            .filter(|(px, _, pixel)| {
                x.contains(px) && pixel.0 == [color[0], color[1], color[2], 255]
            })
            .count()
    };

    let local = ImageRenderer::render(&scene, 240, 180).unwrap();
    let left_edges = count(&local, red, 0..120);
    assert!(left_edges > 120);
    assert_eq!(count(&local, red, 120..240), 0);

    scene
        .set_silhouettes(Some(Silhouettes {
            color: Color::from(green).0,
            ..Silhouettes::default()
        }))
        .unwrap();
    let both = ImageRenderer::render(&scene, 240, 180).unwrap();
    assert_eq!(count(&both, red, 0..120), left_edges);
    assert_eq!(count(&both, green, 0..120), 0);
    assert!(count(&both, green, 120..240) > 32);

    scene.set("left", left.silhouettes(red, 0.0)).unwrap();
    let disabled = ImageRenderer::render(&scene, 240, 180).unwrap();
    assert_eq!(count(&disabled, red, 0..120), 0);
    assert_eq!(count(&disabled, green, 0..120), 0);

    scene.set_silhouettes(None).unwrap();
    scene
        .set("left", left.opacity(0.45).silhouettes(red, 3.0))
        .unwrap();
    let transparent = ImageRenderer::render(&scene, 240, 180).unwrap();
    assert!(count(&transparent, red, 0..120) > 120);
    assert_eq!(count(&transparent, red, 120..240), 0);

    scene.set("left", left.silhouettes(red, 3.0)).unwrap();
    scene.add(Sphere::new([-1.6, 0.0, 1.2], 1.4)).unwrap();
    let hidden = ImageRenderer::render(&scene, 240, 180).unwrap();
    assert_eq!(count(&hidden, red, 0..240), 0);
}

#[test]
#[ignore = "requires an available OpenGL backend"]
fn multishadow_darkens_the_contact_between_opaque_shapes() {
    let mut scene = Scene::new();
    scene.use_black_background();
    scene.set_lighting(1.0, 0.0, 0.0, 1.0, glam::Vec3::ONE);
    scene.set_camera_view(0.0, 0.0, 0.0, 7.0, [0.0; 3], 40.0);
    scene
        .add(Sphere::new([-1.0, 0.0, 0.0], 1.0).color("#E0E0E0"))
        .unwrap();
    let neighbour = Sphere::new([1.0, 0.0, 0.0], 1.0).color("#E0E0E0");
    scene.set("neighbour", neighbour).unwrap();
    let original = ImageRenderer::render(&scene, 256, 256).unwrap();
    scene
        .set_multishadow(Some(Multishadow { directions: 64 }))
        .unwrap();
    let shadowed = ImageRenderer::render(&scene, 256, 256).unwrap();
    scene.set("neighbour", neighbour.opacity(0.4)).unwrap();
    let transparent = ImageRenderer::render(&scene, 256, 256).unwrap();

    // Compare the same pixels of the left sphere, outside the neighbour's silhouette.
    let contact = |image: &image::RgbaImage| {
        (105..119)
            .flat_map(|x| (116..140).map(move |y| image.get_pixel(x, y).0[0] as f32))
            .sum::<f32>()
            / (14.0 * 24.0)
    };
    let original_contact = contact(&original);
    let shadowed_contact = contact(&shadowed);
    let transparent_contact = contact(&transparent);
    assert!(original_contact > 200.0);
    assert!(
        shadowed_contact < original_contact - 15.0,
        "{shadowed_contact} vs {original_contact}"
    );
    assert!(
        transparent_contact > shadowed_contact + 15.0,
        "{transparent_contact} vs {shadowed_contact}"
    );
}
