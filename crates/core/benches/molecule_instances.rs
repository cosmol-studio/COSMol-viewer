use cosmol_viewer_core::{
    scene::Scene, selection::select, styles::BallAndStick, utils::InstanceGroups,
};
use cosmolkit::Molecule;
use std::{hint::black_box, time::Instant};

const SDF: &str = include_str!("../../../cosmol_viewer/examples/6fi1_ligand.sdf");

fn make_linear_sdf(atom_count: usize) -> String {
    assert!((2..=999).contains(&atom_count));
    let bond_count = atom_count - 1;
    let mut sdf = String::new();

    sdf.push_str("linear_chain\n");
    sdf.push_str("  cosmol_viewer bench\n");
    sdf.push('\n');
    sdf.push_str(&format!(
        "{atom_count:>3}{bond_count:>3}  0  0  0  0  0  0  0  0999 V2000\n"
    ));

    for i in 0..atom_count {
        sdf.push_str(&format!(
            "{:>10.4}{:>10.4}{:>10.4} C   0  0  0  0  0  0  0  0  0  0  0  0\n",
            i as f32 * 1.4,
            0.0,
            0.0
        ));
    }

    for i in 1..atom_count {
        sdf.push_str(&format!("{i:>3}{:>3}  1  0  0  0  0\n", i + 1));
    }

    sdf.push_str("M  END\n$$$$\n");
    sdf
}

fn bench_instances(name: &str, iterations: usize, mut f: impl FnMut() -> InstanceGroups) {
    for _ in 0..10 {
        black_box(f());
    }

    let started = Instant::now();
    let mut sphere_count = 0usize;
    let mut stick_count = 0usize;

    for _ in 0..iterations {
        let groups = black_box(f());
        sphere_count = black_box(groups.spheres.len());
        stick_count = black_box(groups.sticks.len());
    }

    let elapsed = started.elapsed();
    let avg_us = elapsed.as_secs_f64() * 1_000_000.0 / iterations as f64;
    println!(
        "{name}: iterations={iterations} avg_us={avg_us:.3} spheres={sphere_count} sticks={stick_count}"
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let molecule = Molecule::from_sdf(SDF)?;
    let linear_64 = Molecule::from_sdf(&make_linear_sdf(64))?;
    let linear_512 = Molecule::from_sdf(&make_linear_sdf(512))?;

    let project = |molecule: &Molecule| {
        let mut scene = Scene::new();
        scene
            .add((&select(molecule).atoms(), BallAndStick::default()))
            .unwrap();
        scene.get_instances_grouped()
    };

    bench_instances("molecule.projection", 10_000, || {
        project(black_box(&molecule))
    });
    bench_instances("linear_64.projection", 5_000, || {
        project(black_box(&linear_64))
    });
    bench_instances("linear_512.projection", 1_000, || {
        project(black_box(&linear_512))
    });

    let mut scene = Scene::new();
    for i in 0..128 {
        scene.set(
            format!("mol_{i}"),
            (&select(&molecule).atoms(), BallAndStick::default()),
        )?;
    }

    bench_instances("scene.get_instances_grouped/128_molecules", 500, || {
        black_box(&scene).get_instances_grouped()
    });

    Ok(())
}
