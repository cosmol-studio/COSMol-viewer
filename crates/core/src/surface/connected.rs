// Derived from UCSF ChimeraX surface/_surface/connected.cpp and
// surface/src/split.py.
// Copyright 2022 Regents of the University of California.
// Licensed under GNU LGPL version 2.1; see surface/NOTICE.

use glam::Vec3;

use super::contour::ContourMesh;

struct Piece {
    vertices: Vec<usize>,
    triangles: Vec<usize>,
}

pub(crate) fn remove_distant_components(
    mesh: ContourMesh,
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    probe_radius: f32,
) -> ContourMesh {
    let pieces = connected_pieces(&mesh.triangles);
    if pieces.is_empty() {
        return mesh;
    }

    let maximum_radius = atom_radii.iter().copied().fold(0.0f32, f32::max);
    let search_radius = 1.5 * probe_radius + maximum_radius;
    let search_radius_squared = search_radius * search_radius;
    let mut keep_vertices = Vec::new();
    let mut keep_triangles = Vec::new();

    for piece in pieces {
        let point = mesh.vertices[piece.vertices[0]];
        let mut nearest = None;
        let mut nearest_distance_squared = f32::INFINITY;
        for (atom_index, &atom_position) in atom_positions.iter().enumerate() {
            let distance_squared = point.distance_squared(atom_position);
            if distance_squared <= search_radius_squared
                && distance_squared < nearest_distance_squared
            {
                nearest = Some(atom_index);
                nearest_distance_squared = distance_squared;
            }
        }
        let Some(atom_index) = nearest else {
            continue;
        };
        let atom_surface_distance = nearest_distance_squared.sqrt() - atom_radii[atom_index];
        if atom_surface_distance < 1.5 * probe_radius {
            keep_vertices.extend(piece.vertices);
            keep_triangles.extend(piece.triangles);
        }
    }

    reduce_geometry(mesh, &keep_vertices, &keep_triangles)
}

fn connected_pieces(triangles: &[[u32; 3]]) -> Vec<Piece> {
    if triangles.is_empty() {
        return Vec::new();
    }

    let vertex_count = triangles
        .iter()
        .flat_map(|triangle| triangle.iter())
        .copied()
        .max()
        .unwrap_or(0) as usize
        + 1;
    let mut vertex_map = vec![vertex_count; vertex_count];

    for triangle in triangles {
        let v0 = min_connected_vertex(triangle[0] as usize, &mut vertex_map);
        let v1 = min_connected_vertex(triangle[1] as usize, &mut vertex_map);
        let v2 = min_connected_vertex(triangle[2] as usize, &mut vertex_map);
        let minimum = v0.min(v1).min(v2);
        vertex_map[v0] = minimum;
        vertex_map[v1] = minimum;
        vertex_map[v2] = minimum;
    }

    let mut component_count = 0;
    for vertex in 0..vertex_count {
        let mapped = vertex_map[vertex];
        if mapped < vertex {
            vertex_map[vertex] = vertex_map[mapped];
        } else if mapped == vertex {
            vertex_map[vertex] = component_count;
            component_count += 1;
        }
    }

    let mut pieces: Vec<Piece> = (0..component_count)
        .map(|_| Piece {
            vertices: Vec::new(),
            triangles: Vec::new(),
        })
        .collect();
    for (vertex, &component) in vertex_map.iter().enumerate() {
        if component < component_count {
            pieces[component].vertices.push(vertex);
        }
    }
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        pieces[vertex_map[triangle[0] as usize]]
            .triangles
            .push(triangle_index);
    }
    pieces
}

fn min_connected_vertex(vertex: usize, vertex_map: &mut [usize]) -> usize {
    let mut minimum = vertex;
    while vertex_map[minimum] < minimum {
        minimum = vertex_map[minimum];
    }
    let mut current = vertex;
    while current > minimum {
        let next = vertex_map[current];
        vertex_map[current] = minimum;
        current = next;
    }
    minimum
}

fn reduce_geometry(
    mesh: ContourMesh,
    vertex_indices: &[usize],
    triangle_indices: &[usize],
) -> ContourMesh {
    let mut vertex_map = vec![0u32; mesh.vertices.len()];
    let vertices = vertex_indices
        .iter()
        .enumerate()
        .map(|(new_index, &old_index)| {
            vertex_map[old_index] = new_index as u32;
            mesh.vertices[old_index]
        })
        .collect();
    let normals = vertex_indices
        .iter()
        .map(|&old_index| mesh.normals[old_index])
        .collect();
    let triangles = triangle_indices
        .iter()
        .map(|&triangle_index| {
            let triangle = mesh.triangles[triangle_index];
            [
                vertex_map[triangle[0] as usize],
                vertex_map[triangle[1] as usize],
                vertex_map[triangle[2] as usize],
            ]
        })
        .collect();
    ContourMesh {
        vertices,
        normals,
        triangles,
    }
}
