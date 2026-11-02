//! CK-backed molecular glyph generation. Temporary coordinate/index arrays are
//! render inputs, never an independently editable molecule.
use super::input::MoleculeInput;
use super::{DisplayError, center, position};
use crate::shapes::{SphereInstance, StickInstance};
use crate::styles::{Coordinates, MolecularOptions, Representation};
use crate::utils::{InstanceGroups, Material, OutlineInstanceGroup};
use cosmolkit::{self as ck, AtomId, BondId, Element};
use glam::Vec3;
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BondGlyph {
    Single,
    Double,
    Triple,
    Unknown,
    Aromatic,
}
#[derive(Clone, Copy)]
enum MolecularMode {
    BallAndStick,
    Stick,
    Sphere,
}

pub(super) fn element_color(element: &Element) -> Vec3 {
    match element.atomic_number() {
        1 => Vec3::new(1.0, 1.0, 1.0),
        6 => Vec3::new(0.3, 0.3, 0.3),
        7 => Vec3::new(0.2, 0.4, 1.0),
        8 => Vec3::new(1.0, 0.0, 0.0),
        9 | 17 => Vec3::new(0.0, 0.8, 0.0),
        15 => Vec3::new(1.0, 0.5, 0.0),
        16 => Vec3::new(1.0, 1.0, 0.0),
        35 => Vec3::new(0.6, 0.2, 0.2),
        53 => Vec3::new(0.4, 0.0, 0.8),
        _ => Vec3::new(0.8, 0.8, 0.8),
    }
}

pub(super) fn vdw_radius(e: &Element) -> f32 {
    match e.atomic_number() {
        1 => 1.20,
        6 => 1.70,
        7 => 1.55,
        8 => 1.52,
        9 => 1.47,
        15 => 1.80,
        16 => 1.80,
        17 => 1.75,
        35 => 1.85,
        53 => 1.98,
        _ => 1.20,
    }
}

pub(super) fn coordinates(
    molecule: &MoleculeInput,
    choice: Coordinates,
) -> Result<Vec<Vec3>, DisplayError> {
    let xyz = match choice {
        Coordinates::Auto => match molecule.conformers.as_slice() {
            [] => None,
            [conformer] => Some(conformer.coordinates.as_slice()),
            conformers => return Err(DisplayError::AmbiguousCoordinates(conformers.len())),
        },
        Coordinates::TwoD => None,
        Coordinates::ThreeD { id } => Some(
            molecule
                .conformers
                .iter()
                .find(|c| c.id == id)
                .ok_or(DisplayError::MissingConformer(id))?
                .coordinates
                .as_slice(),
        ),
    };
    let positions: Vec<Vec3> = if let Some(xyz) = xyz {
        xyz.iter()
            .copied()
            .map(position)
            .collect::<Result<_, _>>()?
    } else {
        molecule
            .coordinates_2d
            .as_deref()
            .ok_or(DisplayError::MissingCoordinates)?
            .iter()
            .map(|xy| position([xy[0], xy[1], 0.0]))
            .collect::<Result<_, _>>()?
    };
    if positions.len() != molecule.elements.len() {
        return Err(DisplayError::CoordinateCount {
            atoms: molecule.elements.len(),
            coordinates: positions.len(),
        });
    }
    Ok(positions)
}

fn configuration(
    style: Representation,
    selection: &'static str,
) -> Result<(MolecularMode, MolecularOptions), DisplayError> {
    Ok(match style {
        Representation::BallAndStick(style) if selection == "molecule atoms" => {
            (MolecularMode::BallAndStick, style.options)
        }
        Representation::BallAndStick(_) => {
            return Err(DisplayError::IncompatibleStyle {
                style: "BallAndStick",
                selection,
            });
        }
        Representation::Sphere(style) if selection == "molecule atoms" => {
            (MolecularMode::Sphere, style.options)
        }
        Representation::Stick(style) => (MolecularMode::Stick, style.options),
        Representation::Cartoon(_) => {
            return Err(DisplayError::IncompatibleStyle {
                style: "Cartoon",
                selection,
            });
        }
        Representation::Surface(_) => {
            return Err(DisplayError::IncompatibleStyle {
                style: "Surface",
                selection,
            });
        }
        Representation::Sphere(_) => {
            return Err(DisplayError::IncompatibleStyle {
                style: "Sphere",
                selection,
            });
        }
    })
}

pub(super) fn project_atoms(
    molecule: &MoleculeInput,
    ids: &[AtomId],
    style: Representation,
) -> Result<(InstanceGroups, Vec3), DisplayError> {
    let (mode, options) = configuration(style, "molecule atoms")?;
    let mut atom_mask = vec![false; molecule.elements.len()];
    for id in ids {
        let selected = atom_mask
            .get_mut(id.index())
            .ok_or(DisplayError::InvalidEntity {
                kind: "atom",
                index: id.index(),
            })?;
        *selected = true;
    }
    let bond_mask: Vec<_> = molecule
        .bonds
        .iter()
        .map(|bond| atom_mask[bond.begin] && atom_mask[bond.end])
        .collect();
    project(molecule, &atom_mask, &bond_mask, mode, options)
}

pub(super) fn project_bonds(
    molecule: &MoleculeInput,
    ids: &[BondId],
    style: Representation,
) -> Result<(InstanceGroups, Vec3), DisplayError> {
    let (mode, options) = configuration(style, "molecule bonds")?;
    let mut atom_mask = vec![false; molecule.elements.len()];
    let mut bond_mask = vec![false; molecule.bonds.len()];
    for id in ids {
        let bond = molecule
            .bonds
            .get(id.index())
            .ok_or(DisplayError::InvalidEntity {
                kind: "bond",
                index: id.index(),
            })?;
        bond_mask[id.index()] = true;
        atom_mask[bond.begin] = true;
        atom_mask[bond.end] = true;
    }
    project(molecule, &atom_mask, &bond_mask, mode, options)
}

fn project(
    molecule: &MoleculeInput,
    atom_mask: &[bool],
    bond_mask: &[bool],
    mode: MolecularMode,
    options: MolecularOptions,
) -> Result<(InstanceGroups, Vec3), DisplayError> {
    if !atom_mask.iter().any(|x| *x) {
        return Ok((InstanceGroups::default(), Vec3::ZERO));
    }
    let positions = coordinates(molecule, options.coordinates)?;
    let display_center = center(
        positions
            .iter()
            .enumerate()
            .filter(|(i, _)| atom_mask[*i])
            .map(|(_, p)| *p),
    );
    if !options.material.visible {
        return Ok((InstanceGroups::default(), display_center));
    }
    let atom_colors = input_colors(molecule);
    let bond_indices: Vec<_> = molecule
        .bonds
        .iter()
        .map(|bond| [bond.begin, bond.end])
        .collect();
    let glyphs: Vec<_> = molecule
        .bonds
        .iter()
        .map(|bond| match bond.order {
            ck::BondOrder::Single => BondGlyph::Single,
            ck::BondOrder::Double => BondGlyph::Double,
            ck::BondOrder::Triple => BondGlyph::Triple,
            ck::BondOrder::Aromatic => BondGlyph::Aromatic,
            _ => BondGlyph::Unknown,
        })
        .collect();
    let scale = 1.0;
    let mut groups = InstanceGroups {
        spheres: Vec::with_capacity(positions.len()),
        sticks: Vec::with_capacity(bond_indices.len() * 6),
        outlines: Vec::new(),
    };

    let alpha = options.material.opacity.clamp(0.0, 1.0);
    let material = [options.material.roughness, options.material.metallic];
    let aromatic_offsets = aromatic_ring_offsets(&positions, &bond_indices, &glyphs);

    if matches!(mode, MolecularMode::BallAndStick | MolecularMode::Sphere) {
        for (i, pos) in positions.iter().enumerate() {
            if !atom_mask[i] {
                continue;
            }
            let color = atom_color(molecule, i, options.material, atom_colors.as_deref());
            groups.spheres.push(SphereInstance::new(
                [pos[0] * scale, pos[1] * scale, pos[2] * scale],
                vdw_radius(&molecule.elements[i])
                    * if matches!(mode, MolecularMode::Sphere) {
                        1.0
                    } else {
                        0.2
                    }
                    * scale
                    * options.radius_scale,
                [color[0], color[1], color[2], alpha],
                material,
            ));
        }
    }

    if matches!(mode, MolecularMode::BallAndStick | MolecularMode::Stick) {
        for (i, bond) in bond_indices.iter().enumerate() {
            if !bond_mask[i] {
                continue;
            }
            let [a, b] = bond;
            let pos_a = positions[*a];
            let pos_b = positions[*b];

            let bond_type = glyphs.get(i).unwrap_or(&BondGlyph::Single);

            let bond_dir = pos_b - pos_a;
            if bond_dir.length_squared() <= BOND_EPSILON * BOND_EPSILON {
                continue;
            }
            let Some(dir_n) = bond_dir.try_normalize() else {
                continue;
            };

            let color_a = bond_color(molecule, *a, options.material);
            let color_b = bond_color(molecule, *b, options.material);
            let color_a = [color_a[0], color_a[1], color_a[2], alpha];
            let color_b = [color_b[0], color_b[1], color_b[2], alpha];

            let (num_sticks, radius, offset_step) = match (mode, *bond_type) {
                (MolecularMode::Stick, BondGlyph::Single) => (1, CHIMERAX_STICK_RADIUS, 0.0),
                (MolecularMode::Stick, BondGlyph::Double) => {
                    let radius = CHIMERAX_STICK_RADIUS * 0.5;
                    (2, radius, radius * 2.0 + CHIMERAX_MULTIBOND_GAP)
                }
                (MolecularMode::Stick, BondGlyph::Triple) => {
                    let radius = CHIMERAX_STICK_RADIUS / 3.0;
                    (3, radius, radius * 2.0 + CHIMERAX_MULTIBOND_GAP)
                }
                (MolecularMode::Stick, BondGlyph::Aromatic) => (1, CHIMERAX_STICK_RADIUS, 0.0),
                (MolecularMode::Stick, _) => (1, CHIMERAX_STICK_RADIUS, 0.0),
                (_, BondGlyph::Single) => (1, 0.135, 0.22),
                (_, BondGlyph::Double) => (2, 0.09, 0.22),
                (_, BondGlyph::Triple) => (3, 0.05, 0.14),
                (_, BondGlyph::Aromatic) => (1, 0.135, 0.0),
                _ => (1, 0.05, 0.22),
            };
            let off_n = if num_sticks > 1 {
                bond_offset_direction(*a, *b, &positions, &bond_indices, dir_n)
                    .or_else(|| bond_offset_direction(*b, *a, &positions, &bond_indices, dir_n))
                    .unwrap_or_else(|| perpendicular_unit_vector(dir_n))
            } else {
                Vec3::ZERO
            };

            for k in 0..num_sticks {
                let offset_mul = (k as f32 - (num_sticks - 1) as f32 * 0.5) * offset_step;

                let pos_a_k = pos_a + off_n * offset_mul;
                let pos_b_k = pos_b + off_n * offset_mul;
                let midpoint = 0.5 * (pos_a_k + pos_b_k);
                let scaled_pos_a = scaled_point(pos_a_k.to_array(), scale);
                let scaled_pos_b = scaled_point(pos_b_k.to_array(), scale);
                let scaled_midpoint = scaled_point(midpoint.to_array(), scale);
                let scaled_radius = radius * scale * options.radius_scale;

                if matches!(mode, MolecularMode::Stick) {
                    groups.spheres.push(SphereInstance::new(
                        scaled_pos_a,
                        scaled_radius,
                        color_a,
                        material,
                    ));
                    groups.spheres.push(SphereInstance::new(
                        scaled_pos_b,
                        scaled_radius,
                        color_b,
                        material,
                    ));
                }

                groups.sticks.push(StickInstance::new(
                    scaled_pos_a,
                    scaled_midpoint,
                    scaled_radius,
                    color_a,
                    material,
                ));

                groups.sticks.push(StickInstance::new(
                    scaled_pos_b,
                    scaled_midpoint,
                    scaled_radius,
                    color_b,
                    material,
                ));
            }

            if *bond_type == BondGlyph::Aromatic {
                let aromatic_radius = radius * AROMATIC_LINE_RADIUS_SCALE;
                let aromatic_offset = radius + aromatic_radius + AROMATIC_LINE_GAP;
                if let Some(inward) = aromatic_offsets[i] {
                    let pos_a_k = pos_a + inward * aromatic_offset;
                    let pos_b_k = pos_b + inward * aromatic_offset;
                    let midpoint = 0.5 * (pos_a_k + pos_b_k);
                    let scaled_pos_a = scaled_point(pos_a_k.to_array(), scale);
                    let scaled_pos_b = scaled_point(pos_b_k.to_array(), scale);
                    let scaled_midpoint = scaled_point(midpoint.to_array(), scale);
                    let scaled_radius = aromatic_radius * scale * options.radius_scale;

                    if matches!(mode, MolecularMode::Stick) {
                        groups.spheres.push(SphereInstance::new(
                            scaled_pos_a,
                            scaled_radius,
                            color_a,
                            material,
                        ));
                        groups.spheres.push(SphereInstance::new(
                            scaled_pos_b,
                            scaled_radius,
                            color_b,
                            material,
                        ));
                    }

                    groups.sticks.push(StickInstance::new(
                        scaled_pos_a,
                        scaled_midpoint,
                        scaled_radius,
                        color_a,
                        material,
                    ));
                    groups.sticks.push(StickInstance::new(
                        scaled_pos_b,
                        scaled_midpoint,
                        scaled_radius,
                        color_b,
                        material,
                    ));
                }
            }
        }
    }
    if options.outline.enabled && options.outline.width > 0.0 {
        let mut settings = options.outline;
        settings.width *= scale;
        groups.outlines.push(OutlineInstanceGroup {
            settings,
            spheres: groups.spheres.clone(),
            sticks: groups.sticks.clone(),
        });
    }
    Ok((groups, display_center))
}

fn atom_color(
    molecule: &MoleculeInput,
    index: usize,
    material: Material,
    colors: Option<&[Option<Vec3>]>,
) -> Vec3 {
    material
        .color
        .or_else(|| colors.and_then(|colors| colors.get(index).copied().flatten()))
        .unwrap_or_else(|| element_color(&molecule.elements[index]))
}
fn bond_color(molecule: &MoleculeInput, index: usize, material: Material) -> Vec3 {
    material.color.unwrap_or_else(|| {
        let element = molecule.elements[index];
        if element == Element::C {
            Vec3::splat(0.75)
        } else {
            element_color(&element)
        }
    })
}

fn scaled_point(point: [f32; 3], scale: f32) -> [f32; 3] {
    [point[0] * scale, point[1] * scale, point[2] * scale]
}

const BOND_EPSILON: f32 = 1.0e-6;
const CHIMERAX_STICK_RADIUS: f32 = 0.2;
const CHIMERAX_MULTIBOND_GAP: f32 = 0.02;
const AROMATIC_LINE_RADIUS_SCALE: f32 = 0.38;
const AROMATIC_LINE_GAP: f32 = 0.02;

fn perpendicular_unit_vector(dir: Vec3) -> Vec3 {
    let up = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    dir.cross(up).normalize()
}

fn bond_offset_direction(
    atom_index: usize,
    excluded_atom: usize,
    atom_posits: &[Vec3],
    bond_indices: &[[usize; 2]],
    bond_dir: Vec3,
) -> Option<Vec3> {
    bond_indices
        .iter()
        .filter_map(|[a, b]| {
            let neighbor = if *a == atom_index && *b != excluded_atom {
                Some(*b)
            } else if *b == atom_index && *a != excluded_atom {
                Some(*a)
            } else {
                None
            }?;

            let neighbor_dir = atom_posits[neighbor] - atom_posits[atom_index];
            let neighbor_dir = neighbor_dir.try_normalize()?;
            let projected = neighbor_dir - neighbor_dir.dot(bond_dir) * bond_dir;
            projected.try_normalize()
        })
        .next()
}

fn aromatic_ring_offsets(
    atom_posits: &[Vec3],
    bond_indices: &[[usize; 2]],
    bond_types: &[BondGlyph],
) -> Vec<Option<Vec3>> {
    let mut offsets = vec![None; bond_indices.len()];
    let mut adjacency = vec![Vec::<(usize, usize)>::new(); atom_posits.len()];

    for (bond_index, [a, b]) in bond_indices.iter().enumerate() {
        if bond_types.get(bond_index) != Some(&BondGlyph::Aromatic) {
            continue;
        }
        if *a >= atom_posits.len() || *b >= atom_posits.len() {
            continue;
        }
        adjacency[*a].push((*b, bond_index));
        adjacency[*b].push((*a, bond_index));
    }

    for (bond_index, _) in bond_indices.iter().enumerate() {
        if bond_types.get(bond_index) != Some(&BondGlyph::Aromatic) {
            continue;
        }
        offsets[bond_index] =
            aromatic_bond_inner_offset(bond_index, atom_posits, bond_indices, &adjacency);
    }

    offsets
}

fn aromatic_bond_inner_offset(
    bond_index: usize,
    atom_posits: &[Vec3],
    bond_indices: &[[usize; 2]],
    adjacency: &[Vec<(usize, usize)>],
) -> Option<Vec3> {
    let [start, goal] = *bond_indices.get(bond_index)?;
    if start >= atom_posits.len() || goal >= atom_posits.len() {
        return None;
    }

    let mut queue = VecDeque::from([start]);
    let mut visited = vec![false; atom_posits.len()];
    let mut previous = vec![None::<usize>; atom_posits.len()];
    visited[start] = true;

    while let Some(current) = queue.pop_front() {
        for &(next, edge_index) in &adjacency[current] {
            if edge_index == bond_index || visited[next] {
                continue;
            }

            visited[next] = true;
            previous[next] = Some(current);
            if next == goal {
                break;
            }

            queue.push_back(next);
        }
    }

    if !visited[goal] {
        return None;
    }

    let mut cycle_atoms = vec![goal];
    let mut current = goal;
    while current != start {
        current = previous[current]?;
        cycle_atoms.push(current);
    }

    if cycle_atoms.len() < 3 {
        return None;
    }

    let center = cycle_atoms
        .iter()
        .map(|atom| atom_posits[*atom])
        .sum::<Vec3>()
        / cycle_atoms.len() as f32;

    let bond = atom_posits[goal] - atom_posits[start];
    let dir = bond.try_normalize()?;
    let midpoint = 0.5 * (atom_posits[start] + atom_posits[goal]);
    let inward = center - midpoint;
    let projected = inward - dir * inward.dot(dir);
    projected.try_normalize()
}

fn color_from_weight(weight: f64) -> Vec3 {
    Vec3::new(weight as f32, 1.0 - weight as f32, 0.0)
}

fn parse_f64_lines(value: &str) -> Option<Vec<f64>> {
    value
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn parse_usize_lines(value: &str) -> Option<Vec<usize>> {
    value
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn parse_dummy_atom_weights(
    indices_value: Option<&str>,
    weights_value: &str,
    atom_count: usize,
) -> Option<Vec<Option<Vec3>>> {
    let weights = parse_f64_lines(weights_value)?;

    if let Some(indices_value) = indices_value {
        let indices = parse_usize_lines(indices_value)?;
        if indices.len() != weights.len() {
            eprintln!(
                "[WARN] Ignoring DUMMY_ATOM_INDICES/DUMMY_WEIGHTS: expected matching lengths, got {} indices and {} weights",
                indices.len(),
                weights.len()
            );
            return None;
        }

        let mut colors = vec![None; atom_count];
        for (atom_index, weight) in indices.into_iter().zip(weights) {
            if atom_index == 0 || atom_index > atom_count {
                eprintln!(
                    "[WARN] Ignoring DUMMY_ATOM_INDICES/DUMMY_WEIGHTS: atom index {atom_index} out of range 1..={atom_count}"
                );
                return None;
            }
            colors[atom_index - 1] = Some(color_from_weight(weight));
        }

        return Some(colors);
    }

    if weights.len() != atom_count {
        eprintln!(
            "[WARN] Ignoring DUMMY_WEIGHTS: expected {atom_count} values for all atoms, got {}",
            weights.len()
        );
        return None;
    }

    Some(
        weights
            .into_iter()
            .map(|weight| Some(color_from_weight(weight)))
            .collect(),
    )
}

fn input_colors(input: &MoleculeInput) -> Option<Vec<Option<Vec3>>> {
    if let Some(colors) = input.dummy_weights.as_deref().and_then(|weights| {
        parse_dummy_atom_weights(
            input.dummy_indices.as_deref(),
            weights,
            input.elements.len(),
        )
    }) {
        return Some(colors);
    }
    let colors: Vec<_> = input
        .atom_weights
        .iter()
        .map(|w| w.map(color_from_weight))
        .collect();
    colors.iter().any(Option::is_some).then_some(colors)
}
