// Derived from UCSF ChimeraX surface/_surface/patches.cpp.
// Copyright 2022 Regents of the University of California.
// Licensed under GNU LGPL version 2.1; see surface/NOTICE.

use std::collections::BTreeMap;

use glam::{DVec3, Vec3};

use super::SurfaceGeometry;

type Edge = (usize, usize);
type EdgeMap = BTreeMap<Edge, usize>;

struct PatchMesh {
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    vertex_atoms: Vec<usize>,
    vertex_map: Vec<usize>,
}

pub(crate) fn sharp_edge_patches(
    geometry: SurfaceGeometry,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    probe_radius: f32,
    grid_spacing: f32,
    refinement_steps: usize,
) -> SurfaceGeometry {
    let max_radius = atom_radii.iter().copied().fold(0.0f32, f32::max);
    let max_distance = 1.1 * (probe_radius + max_radius + grid_spacing);
    let vertex_atoms =
        vertex_to_atom_map(&geometry.vertices, atom_positions, atom_radii, max_distance);
    let vertex_count = geometry.vertices.len();
    let mut mesh = PatchMesh {
        vertices: geometry.vertices,
        normals: geometry.normals,
        triangles: geometry.triangles,
        vertex_atoms,
        vertex_map: (0..vertex_count).collect(),
    };
    sharp_patches(&mut mesh, atom_positions, atom_radii, refinement_steps);
    SurfaceGeometry {
        vertices: mesh.vertices,
        normals: mesh.normals,
        triangles: mesh.triangles,
    }
}

fn vertex_to_atom_map(
    vertices: &[Vec3],
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    max_distance: f32,
) -> Vec<usize> {
    let max_distance_squared = max_distance * max_distance;
    vertices
        .iter()
        .map(|&vertex| {
            let mut nearest = None;
            let mut nearest_distance_squared = 0.0f32;
            for (atom_index, (&atom, &radius)) in atom_positions.iter().zip(atom_radii).enumerate()
            {
                let distance_squared = vertex.distance_squared(atom);
                if distance_squared > max_distance_squared {
                    continue;
                }
                let replace = match nearest {
                    None => true,
                    Some(current) => {
                        let current_radius = atom_radii[current];
                        current_radius * current_radius * distance_squared
                            < radius * radius * nearest_distance_squared
                    }
                };
                if replace {
                    nearest = Some(atom_index);
                    nearest_distance_squared = distance_squared;
                }
            }
            nearest.expect("surface vertex is farther from atoms than ChimeraX maximum distance")
        })
        .collect()
}

fn sharp_patches(
    mesh: &mut PatchMesh,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    refinement_steps: usize,
) {
    let mut edge_splits = EdgeMap::new();
    compute_edge_split_points(mesh, atom_positions, atom_radii, &mut edge_splits);
    for _ in 0..refinement_steps {
        if refine_3_patch_triangles(mesh, atom_positions, atom_radii, &mut edge_splits) == 0 {
            break;
        }
    }
    divide_triangles(mesh, atom_positions, &mut edge_splits);
}

fn split_fraction(p0: Vec3, p1: Vec3, atom0: Vec3, atom1: Vec3) -> f64 {
    let p0 = vec3_to_dvec3(p0);
    let p1 = vec3_to_dvec3(p1);
    let atom0 = vec3_to_dvec3(atom0);
    let atom1 = vec3_to_dvec3(atom1);
    let direction = p1 - p0;
    let atom_direction = atom1 - atom0;
    let center_offset = 0.5 * (atom0 + atom1) - p0;
    let denominator = direction.dot(atom_direction);
    if denominator != 0.0 {
        center_offset.dot(atom_direction) / denominator
    } else {
        0.5
    }
}

fn scaled_split_fraction(
    p0: Vec3,
    p1: Vec3,
    atom0: Vec3,
    atom1: Vec3,
    radius0: f32,
    radius1: f32,
) -> f64 {
    let p0 = vec3_to_dvec3(p0);
    let p1 = vec3_to_dvec3(p1);
    let atom0 = vec3_to_dvec3(atom0);
    let atom1 = vec3_to_dvec3(atom1);
    let direction = p1 - p0;
    let d2 = direction.length_squared();
    let radius0_squared = f64::from(radius0) * f64::from(radius0);
    let radius1_squared = f64::from(radius1) * f64::from(radius1);
    let a = d2 * (radius1_squared - radius0_squared);
    if a == 0.0 {
        return split_fraction(p0.as_vec3(), p1.as_vec3(), atom0.as_vec3(), atom1.as_vec3());
    }
    let error0 = p0 - atom0;
    let error1 = p0 - atom1;
    let b = radius1_squared * direction.dot(error0) - radius0_squared * direction.dot(error1);
    let c = radius1_squared * error0.length_squared() - radius0_squared * error1.length_squared();
    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        0.5
    } else {
        (-b + discriminant.sqrt()) / a
    }
}

fn split_point(
    vertex0: usize,
    vertex1: usize,
    atom0: usize,
    atom1: usize,
    vertices: &[Vec3],
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    clamp: bool,
) -> (Vec3, f32) {
    if vertex0 > vertex1 {
        let (point, fraction) = split_point(
            vertex1,
            vertex0,
            atom1,
            atom0,
            vertices,
            atom_positions,
            atom_radii,
            clamp,
        );
        return (point, 1.0 - fraction);
    }
    let mut fraction = scaled_split_fraction(
        vertices[vertex0],
        vertices[vertex1],
        atom_positions[atom0],
        atom_positions[atom1],
        atom_radii[atom0],
        atom_radii[atom1],
    );
    if clamp {
        fraction = fraction.clamp(0.0, 1.0);
    }
    let point0 = vec3_to_dvec3(vertices[vertex0]);
    let point1 = vec3_to_dvec3(vertices[vertex1]);
    let point = point0 * (1.0 - fraction) + point1 * fraction;
    let fraction = fraction as f32;
    (point.as_vec3(), fraction)
}

#[inline]
fn vec3_to_dvec3(value: Vec3) -> DVec3 {
    DVec3::new(f64::from(value.x), f64::from(value.y), f64::from(value.z))
}

fn split_edge(
    mesh: &mut PatchMesh,
    vertex0: usize,
    vertex1: usize,
    atom0: usize,
    atom1: usize,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
) -> usize {
    let (point, fraction) = split_point(
        vertex0,
        vertex1,
        atom0,
        atom1,
        &mesh.vertices,
        atom_positions,
        atom_radii,
        true,
    );
    let vertex = mesh.vertices.len();
    mesh.vertex_map.push(vertex);
    mesh.vertices.push(point);
    let normal = (mesh.normals[vertex0] * (1.0 - fraction) + mesh.normals[vertex1] * fraction)
        .normalize_or_zero();
    mesh.normals.push(normal);
    mesh.vertex_atoms.push(atom0);
    vertex
}

fn add_split_point(
    mesh: &mut PatchMesh,
    vertex0: usize,
    vertex1: usize,
    atom0: usize,
    atom1: usize,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    edge_splits: &mut EdgeMap,
) {
    let (minimum, maximum, atom_minimum, atom_maximum) = if vertex0 < vertex1 {
        (vertex0, vertex1, atom0, atom1)
    } else {
        (vertex1, vertex0, atom1, atom0)
    };
    edge_splits.entry((minimum, maximum)).or_insert_with(|| {
        split_edge(
            mesh,
            minimum,
            maximum,
            atom_minimum,
            atom_maximum,
            atom_positions,
            atom_radii,
        )
    });
}

fn compute_edge_split_points(
    mesh: &mut PatchMesh,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    edge_splits: &mut EdgeMap,
) {
    let triangles = mesh.triangles.clone();
    for triangle in triangles {
        let [v0, v1, v2] = triangle.map(|vertex| vertex as usize);
        let [a0, a1, a2] = [
            mesh.vertex_atoms[v0],
            mesh.vertex_atoms[v1],
            mesh.vertex_atoms[v2],
        ];
        if a0 != a1 {
            add_split_point(
                mesh,
                v0,
                v1,
                a0,
                a1,
                atom_positions,
                atom_radii,
                edge_splits,
            );
        }
        if a1 != a2 {
            add_split_point(
                mesh,
                v1,
                v2,
                a1,
                a2,
                atom_positions,
                atom_radii,
                edge_splits,
            );
        }
        if a2 != a0 {
            add_split_point(
                mesh,
                v2,
                v0,
                a2,
                a0,
                atom_positions,
                atom_radii,
                edge_splits,
            );
        }
    }
}

fn edge_vertex(edge_splits: &EdgeMap, vertex0: usize, vertex1: usize) -> usize {
    edge_splits[&(vertex0, vertex1)]
}

fn add_triangle(triangles: &mut Vec<[u32; 3]>, v0: usize, v1: usize, v2: usize) {
    assert!(v0 != v1 && v1 != v2 && v2 != v0);
    triangles.push([v0 as u32, v1 as u32, v2 as u32]);
}

fn cut_triangle_1_line(
    mut v0: usize,
    mut v1: usize,
    mut v2: usize,
    a0: usize,
    a1: usize,
    a2: usize,
    triangles: &mut Vec<[u32; 3]>,
    edge_splits: &EdgeMap,
) {
    if a1 == a2 {
        (v0, v1, v2) = (v1, v2, v0);
    } else if a2 == a0 {
        (v0, v1, v2) = (v2, v0, v1);
    }
    let v12 = edge_vertex(edge_splits, v1, v2);
    let v21 = edge_vertex(edge_splits, v2, v1);
    let v20 = edge_vertex(edge_splits, v2, v0);
    let v02 = edge_vertex(edge_splits, v0, v2);
    add_triangle(triangles, v0, v1, v12);
    add_triangle(triangles, v0, v12, v02);
    add_triangle(triangles, v2, v20, v21);
}

fn compute_triple_point(
    mesh: &PatchMesh,
    vertices: [usize; 3],
    atoms: [usize; 3],
    split01: usize,
    split02: usize,
    atom_positions: &[Vec3],
) -> (f32, f32, Vec3, Vec3) {
    let [v0, v1, v2] = vertices.map(|vertex| mesh.vertices[vertex]);
    let [a0, a1, a2] = atoms.map(|atom| atom_positions[atom]);
    let v01 = v1 - v0;
    let v02 = v2 - v0;
    let atom01 = a1 - a0;
    let atom02 = a2 - a0;
    let middle01 = mesh.vertices[split01] - v0;
    let middle02 = mesh.vertices[split02] - v0;

    let v01_atom01 = v01.dot(atom01);
    let v01_atom02 = v01.dot(atom02);
    let v02_atom01 = v02.dot(atom01);
    let v02_atom02 = v02.dot(atom02);
    let middle01_atom01 = middle01.dot(atom01);
    let middle02_atom02 = middle02.dot(atom02);
    let denominator = v01_atom01 * v02_atom02 - v02_atom01 * v01_atom02;
    let fraction1 = if denominator != 0.0 {
        (v02_atom02 * middle01_atom01 - v02_atom01 * middle02_atom02) / denominator
    } else {
        0.0
    };
    let fraction2 = if denominator != 0.0 {
        (v01_atom01 * middle02_atom02 - v01_atom02 * middle01_atom01) / denominator
    } else {
        0.0
    };
    let normal = (mesh.normals[vertices[0]]
        + fraction1 * (mesh.normals[vertices[1]] - mesh.normals[vertices[0]])
        + fraction2 * (mesh.normals[vertices[2]] - mesh.normals[vertices[0]]))
        .normalize_or_zero();
    (
        fraction1,
        fraction2,
        v0 + fraction1 * v01 + fraction2 * v02,
        normal,
    )
}

#[allow(clippy::too_many_arguments)]
fn cut_to_vertex(
    mesh: &mut PatchMesh,
    v0: usize,
    v1: usize,
    v2: usize,
    a1: usize,
    a2: usize,
    v10: usize,
    v12: usize,
    v21: usize,
    v20: usize,
    triangles: &mut Vec<[u32; 3]>,
) {
    let v012 = mesh.vertices.len();
    let v021 = v012 + 1;
    mesh.vertices.push(mesh.vertices[v0]);
    mesh.vertices.push(mesh.vertices[v0]);
    mesh.vertex_map.push(v0);
    mesh.vertex_map.push(v0);
    mesh.normals.push(mesh.normals[v0]);
    mesh.normals.push(mesh.normals[v0]);
    mesh.vertex_atoms.push(a1);
    mesh.vertex_atoms.push(a2);
    add_triangle(triangles, v012, v10, v12);
    add_triangle(triangles, v10, v1, v12);
    add_triangle(triangles, v021, v21, v20);
    add_triangle(triangles, v2, v20, v21);
}

#[allow(clippy::too_many_arguments)]
fn cut_to_edge(
    mesh: &mut PatchMesh,
    v0: usize,
    v1: usize,
    v2: usize,
    a2: usize,
    v01: usize,
    v10: usize,
    v12: usize,
    v21: usize,
    v20: usize,
    v02: usize,
    triangles: &mut Vec<[u32; 3]>,
) {
    let v012 = mesh.vertices.len();
    mesh.vertices.push(mesh.vertices[v01]);
    mesh.vertex_map.push(v01);
    mesh.normals.push(mesh.normals[v01]);
    mesh.vertex_atoms.push(a2);
    add_triangle(triangles, v0, v01, v02);
    add_triangle(triangles, v1, v12, v10);
    add_triangle(triangles, v2, v20, v012);
    add_triangle(triangles, v2, v012, v21);
}

#[allow(clippy::too_many_arguments)]
fn cut_to_middle(
    mesh: &mut PatchMesh,
    point: Vec3,
    normal: Vec3,
    vertices: [usize; 3],
    atoms: [usize; 3],
    splits: [usize; 6],
    triangles: &mut Vec<[u32; 3]>,
) {
    let center0 = mesh.vertices.len();
    for _ in 0..3 {
        mesh.vertices.push(point);
        mesh.vertex_map.push(center0);
        mesh.normals.push(normal);
    }
    mesh.vertex_atoms.extend(atoms);
    let [v0, v1, v2] = vertices;
    let [v01, v10, v12, v21, v20, v02] = splits;
    add_triangle(triangles, v0, v01, center0);
    add_triangle(triangles, v0, center0, v02);
    add_triangle(triangles, v1, center0 + 1, v10);
    add_triangle(triangles, v1, v12, center0 + 1);
    add_triangle(triangles, v2, center0 + 2, v21);
    add_triangle(triangles, v2, v20, center0 + 2);
}

fn cut_triangle_3_lines(
    mesh: &mut PatchMesh,
    vertices: [usize; 3],
    atoms: [usize; 3],
    atom_positions: &[Vec3],
    triangles: &mut Vec<[u32; 3]>,
    edge_splits: &EdgeMap,
) {
    let [v0, v1, v2] = vertices;
    let [a0, a1, a2] = atoms;
    let splits = [
        edge_vertex(edge_splits, v0, v1),
        edge_vertex(edge_splits, v1, v0),
        edge_vertex(edge_splits, v1, v2),
        edge_vertex(edge_splits, v2, v1),
        edge_vertex(edge_splits, v2, v0),
        edge_vertex(edge_splits, v0, v2),
    ];
    let (fraction1, fraction2, point, normal) =
        compute_triple_point(mesh, vertices, atoms, splits[0], splits[5], atom_positions);
    let fraction12 = fraction1 + fraction2;
    if fraction1 > 0.0 && fraction2 > 0.0 && fraction12 < 1.0 {
        cut_to_middle(mesh, point, normal, vertices, atoms, splits, triangles);
        return;
    }
    let outside = usize::from(fraction1 <= 0.0)
        + usize::from(fraction2 <= 0.0)
        + usize::from(fraction12 >= 1.0);
    if outside == 1 {
        if fraction1 <= 0.0 {
            cut_to_edge(
                mesh, v2, v0, v1, a1, splits[4], splits[5], splits[0], splits[1], splits[2],
                splits[3], triangles,
            );
        } else if fraction2 <= 0.0 {
            cut_to_edge(
                mesh, v0, v1, v2, a2, splits[0], splits[1], splits[2], splits[3], splits[4],
                splits[5], triangles,
            );
        } else {
            cut_to_edge(
                mesh, v1, v2, v0, a0, splits[2], splits[3], splits[4], splits[5], splits[0],
                splits[1], triangles,
            );
        }
    } else if outside == 2 {
        if fraction1 > 0.0 {
            cut_to_vertex(
                mesh, v1, v2, v0, a2, a0, splits[3], splits[4], splits[5], splits[0], triangles,
            );
        } else if fraction2 > 0.0 {
            cut_to_vertex(
                mesh, v2, v0, v1, a0, a1, splits[5], splits[0], splits[1], splits[2], triangles,
            );
        } else {
            cut_to_vertex(
                mesh, v0, v1, v2, a1, a2, splits[1], splits[2], splits[3], splits[4], triangles,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn three_patch_edge(
    mesh: &mut PatchMesh,
    vertex1: usize,
    vertex2: usize,
    atom1: usize,
    atom0: usize,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    edge_splits: &EdgeMap,
    edge_3p: &mut EdgeMap,
) -> bool {
    let edge = ordered_edge(vertex1, vertex2);
    let split = edge_vertex(edge_splits, edge.0, edge.1);
    let distance0 = mesh.vertices[split].distance_squared(atom_positions[atom0]);
    let distance1 = mesh.vertices[split].distance_squared(atom_positions[atom1]);
    let divide = distance0 * atom_radii[atom1] * atom_radii[atom1]
        < distance1 * atom_radii[atom0] * atom_radii[atom0];
    if divide {
        edge_3p.insert(edge, split);
        mesh.vertex_atoms[split] = atom0;
    }
    divide
}

fn split_triangle_1_edge(
    v0: usize,
    v1: usize,
    v2: usize,
    v01: usize,
    triangles: &mut Vec<[u32; 3]>,
) {
    add_triangle(triangles, v0, v01, v2);
    add_triangle(triangles, v1, v2, v01);
}

fn split_triangle_2_edges(
    v0: usize,
    v1: usize,
    v2: usize,
    v01: usize,
    v12: usize,
    triangles: &mut Vec<[u32; 3]>,
) {
    add_triangle(triangles, v0, v01, v2);
    add_triangle(triangles, v1, v12, v01);
    add_triangle(triangles, v2, v01, v12);
}

fn split_triangle_3_edges(
    v0: usize,
    v1: usize,
    v2: usize,
    v01: usize,
    v12: usize,
    v20: usize,
    triangles: &mut Vec<[u32; 3]>,
) {
    add_triangle(triangles, v0, v01, v20);
    add_triangle(triangles, v1, v12, v01);
    add_triangle(triangles, v2, v20, v12);
    add_triangle(triangles, v01, v12, v20);
}

fn split_triangle(
    v0: usize,
    v1: usize,
    v2: usize,
    v01: Option<usize>,
    v12: Option<usize>,
    v20: Option<usize>,
    triangles: &mut Vec<[u32; 3]>,
) {
    match (v01, v12, v20) {
        (Some(v01), None, None) => split_triangle_1_edge(v0, v1, v2, v01, triangles),
        (None, Some(v12), None) => split_triangle_1_edge(v1, v2, v0, v12, triangles),
        (None, None, Some(v20)) => split_triangle_1_edge(v2, v0, v1, v20, triangles),
        (Some(v01), Some(v12), None) => split_triangle_2_edges(v0, v1, v2, v01, v12, triangles),
        (None, Some(v12), Some(v20)) => split_triangle_2_edges(v1, v2, v0, v12, v20, triangles),
        (Some(v01), None, Some(v20)) => split_triangle_2_edges(v2, v0, v1, v20, v01, triangles),
        (Some(v01), Some(v12), Some(v20)) => {
            split_triangle_3_edges(v0, v1, v2, v01, v12, v20, triangles)
        }
        (None, None, None) => {}
    }
}

fn minimize_atom_distance(
    mesh: &mut PatchMesh,
    vertex0: usize,
    vertex1: usize,
    atom0: usize,
    atom1: usize,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
) -> usize {
    if atom0 == atom1 {
        return 0;
    }
    let distance10 = distance_squared_f64(mesh.vertices[vertex1], atom_positions[atom0]);
    let distance11 = distance_squared_f64(mesh.vertices[vertex1], atom_positions[atom1]);
    let distance00 = distance_squared_f64(mesh.vertices[vertex0], atom_positions[atom0]);
    let distance01 = distance_squared_f64(mesh.vertices[vertex0], atom_positions[atom1]);
    let radius0 = f64::from(atom_radii[atom0]);
    let radius1 = f64::from(atom_radii[atom1]);
    let mut changes = 0;
    if distance10 * radius1 * radius1 < distance11 * radius0 * radius0 {
        mesh.vertex_atoms[vertex1] = atom0;
        changes += 1;
    }
    if distance01 * radius0 * radius0 < distance00 * radius1 * radius1 {
        mesh.vertex_atoms[vertex0] = atom1;
        changes += 1;
    }
    let _ = split_point(
        vertex0,
        vertex1,
        mesh.vertex_atoms[vertex0],
        mesh.vertex_atoms[vertex1],
        &mesh.vertices,
        atom_positions,
        atom_radii,
        false,
    );
    changes
}

#[inline]
fn distance_squared_f64(point0: Vec3, point1: Vec3) -> f64 {
    // ChimeraX subtracts float coordinates, then accumulates their squares in double.
    let dx = f64::from(point0.x - point1.x);
    let dy = f64::from(point0.y - point1.y);
    let dz = f64::from(point0.z - point1.z);
    dx * dx + dy * dy + dz * dz
}

fn minimize_atom_distances(
    mesh: &mut PatchMesh,
    triangles: &[[u32; 3]],
    atom_positions: &[Vec3],
    atom_radii: &[f32],
) -> usize {
    let mut changes = 0;
    for triangle in triangles {
        let [v0, v1, v2] = triangle.map(|vertex| vertex as usize);
        let [a0, a1, a2] = [
            mesh.vertex_atoms[v0],
            mesh.vertex_atoms[v1],
            mesh.vertex_atoms[v2],
        ];
        changes += minimize_atom_distance(mesh, v0, v1, a0, a1, atom_positions, atom_radii);
        changes += minimize_atom_distance(mesh, v1, v2, a1, a2, atom_positions, atom_radii);
        changes += minimize_atom_distance(mesh, v2, v0, a2, a0, atom_positions, atom_radii);
    }
    changes
}

fn find_3_patch_edges(
    mesh: &mut PatchMesh,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    edge_splits: &EdgeMap,
) -> EdgeMap {
    let triangles = mesh.triangles.clone();
    let mut edge_3p = EdgeMap::new();
    for triangle in triangles {
        let [v0, v1, v2] = triangle.map(|vertex| vertex as usize);
        let [a0, a1, a2] = [
            mesh.vertex_atoms[v0],
            mesh.vertex_atoms[v1],
            mesh.vertex_atoms[v2],
        ];
        if a0 != a1 && a1 != a2 && a2 != a0 {
            three_patch_edge(
                mesh,
                v0,
                v1,
                a0,
                a2,
                atom_positions,
                atom_radii,
                edge_splits,
                &mut edge_3p,
            );
            three_patch_edge(
                mesh,
                v1,
                v2,
                a1,
                a0,
                atom_positions,
                atom_radii,
                edge_splits,
                &mut edge_3p,
            );
            three_patch_edge(
                mesh,
                v2,
                v0,
                a2,
                a1,
                atom_positions,
                atom_radii,
                edge_splits,
                &mut edge_3p,
            );
        }
    }
    edge_3p
}

fn split_3_patch_triangles(mesh: &PatchMesh, edge_3p: &EdgeMap) -> (Vec<[u32; 3]>, Vec<[u32; 3]>) {
    let mut split = Vec::new();
    let mut unsplit = Vec::new();
    for triangle in &mesh.triangles {
        let [v0, v1, v2] = triangle.map(|vertex| vertex as usize);
        let [a0, a1, a2] = [
            mesh.vertex_atoms[v0],
            mesh.vertex_atoms[v1],
            mesh.vertex_atoms[v2],
        ];
        let v01 = (a0 != a1)
            .then(|| edge_3p.get(&ordered_edge(v0, v1)).copied())
            .flatten();
        let v12 = (a1 != a2)
            .then(|| edge_3p.get(&ordered_edge(v1, v2)).copied())
            .flatten();
        let v20 = (a2 != a0)
            .then(|| edge_3p.get(&ordered_edge(v2, v0)).copied())
            .flatten();
        if v01.is_some() || v12.is_some() || v20.is_some() {
            split_triangle(v0, v1, v2, v01, v12, v20, &mut split);
        } else {
            add_triangle(&mut unsplit, v0, v1, v2);
        }
    }
    (split, unsplit)
}

fn refine_3_patch_triangles(
    mesh: &mut PatchMesh,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    edge_splits: &mut EdgeMap,
) -> usize {
    let edge_3p = find_3_patch_edges(mesh, atom_positions, atom_radii, edge_splits);
    let (split, unsplit) = split_3_patch_triangles(mesh, &edge_3p);
    for edge in edge_3p.keys() {
        edge_splits.remove(edge);
    }

    for _ in 0..10 {
        let changes = minimize_atom_distances(mesh, &split, atom_positions, atom_radii)
            + minimize_atom_distances(mesh, &unsplit, atom_positions, atom_radii);
        if changes == 0 {
            break;
        }
    }

    mesh.triangles = split.clone();
    compute_edge_split_points(mesh, atom_positions, atom_radii, edge_splits);
    mesh.triangles = unsplit.clone();
    compute_edge_split_points(mesh, atom_positions, atom_radii, edge_splits);

    let split_count = split.len();
    mesh.triangles = unsplit;
    mesh.triangles.extend(split);
    split_count
}

fn duplicate_edge_vertices(mesh: &mut PatchMesh, edge_splits: &mut EdgeMap) {
    let mut duplicates = EdgeMap::new();
    for (&edge, &edge_vertex) in edge_splits.iter() {
        let vertex = mesh.vertices.len();
        duplicates.insert((edge.1, edge.0), vertex);
        mesh.vertices.push(mesh.vertices[edge_vertex]);
        mesh.vertex_map.push(edge_vertex);
        mesh.normals.push(mesh.normals[edge_vertex]);
        mesh.vertex_atoms.push(mesh.vertex_atoms[edge.1]);
        mesh.vertex_atoms[edge_vertex] = mesh.vertex_atoms[edge.0];
    }
    edge_splits.extend(duplicates);
}

fn divide_triangles(mesh: &mut PatchMesh, atom_positions: &[Vec3], edge_splits: &mut EdgeMap) {
    duplicate_edge_vertices(mesh, edge_splits);
    let triangles = mesh.triangles.clone();
    let mut divided = Vec::new();
    for triangle in triangles {
        let [v0, v1, v2] = triangle.map(|vertex| vertex as usize);
        let [a0, a1, a2] = [
            mesh.vertex_atoms[v0],
            mesh.vertex_atoms[v1],
            mesh.vertex_atoms[v2],
        ];
        if a0 == a1 && a1 == a2 {
            add_triangle(&mut divided, v0, v1, v2);
        } else if a0 != a1 && a1 != a2 && a2 != a0 {
            cut_triangle_3_lines(
                mesh,
                [v0, v1, v2],
                [a0, a1, a2],
                atom_positions,
                &mut divided,
                edge_splits,
            );
        } else {
            cut_triangle_1_line(v0, v1, v2, a0, a1, a2, &mut divided, edge_splits);
        }
    }
    mesh.triangles = divided;
}

#[inline]
fn ordered_edge(vertex0: usize, vertex1: usize) -> Edge {
    (vertex0.min(vertex1), vertex0.max(vertex1))
}
