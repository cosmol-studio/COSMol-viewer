use cosmol_viewer_core::{
    scene::Scene,
    shapes::Molecule,
    utils::{InstanceGroups, IntoInstanceGroups},
};
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
    let molecule = Molecule::from_sdf(SDF)?.centered();
    let linear_64 = Molecule::from_sdf(&make_linear_sdf(64))?.centered();
    let linear_512 = Molecule::from_sdf(&make_linear_sdf(512))?.centered();

    bench_instances("molecule.to_instance_group", 10_000, || {
        black_box(&molecule).to_instance_group(black_box(1.0))
    });
    bench_instances("linear_64.to_instance_group", 5_000, || {
        black_box(&linear_64).to_instance_group(black_box(1.0))
    });
    bench_instances("linear_512.to_instance_group", 1_000, || {
        black_box(&linear_512).to_instance_group(black_box(1.0))
    });

    let mut scene = Scene::new();
    for i in 0..128 {
        scene.add_shape_with_id(format!("mol_{i}"), molecule.clone());
    }

    bench_instances("scene.get_instances_grouped/128_molecules", 500, || {
        black_box(&scene).get_instances_grouped()
    });

    Ok(())
}
