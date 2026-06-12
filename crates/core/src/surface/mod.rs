// Derived from UCSF ChimeraX surface/src/gridsurf.py.
// Copyright 2022 Regents of the University of California.
// Licensed under GNU LGPL version 2.1; see surface/NOTICE.

mod connected;
mod contour;
mod contourdata;
mod distance_grid;
mod patches;
pub(crate) mod protein_templates;
pub(crate) mod radii;

use glam::Vec3;
use thiserror::Error;

use connected::remove_distant_components;
use contour::{ContourMesh, contour_surface};
use distance_grid::{Grid, sphere_surface_distance};
pub(crate) use patches::sharp_edge_patches;

#[derive(Debug, Error)]
pub enum SurfaceError {
    #[error("surface requires at least one atom")]
    EmptyAtoms,
    #[error("atom position and radius counts do not match")]
    AtomCountMismatch,
    #[error("probe radius must be finite and non-negative")]
    InvalidProbeRadius,
    #[error("grid spacing must be finite and greater than zero")]
    InvalidGridSpacing,
    #[error("surface grid dimensions overflowed or could not be allocated: {0:?}")]
    GridAllocation([usize; 3]),
}

#[derive(Debug, Clone)]
pub struct SurfaceGeometry {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

pub fn ses_surface_geometry(
    atom_positions: &[Vec3],
    atom_radii: &[f32],
    probe_radius: f32,
    grid_spacing: f32,
    solvent_accessible: bool,
) -> Result<SurfaceGeometry, SurfaceError> {
    if atom_positions.is_empty() {
        return Err(SurfaceError::EmptyAtoms);
    }
    if atom_positions.len() != atom_radii.len() {
        return Err(SurfaceError::AtomCountMismatch);
    }
    if !probe_radius.is_finite() || probe_radius < 0.0 {
        return Err(SurfaceError::InvalidProbeRadius);
    }
    if !grid_spacing.is_finite() || grid_spacing <= 0.0 {
        return Err(SurfaceError::InvalidGridSpacing);
    }

    let mut minimum = atom_positions[0];
    let mut maximum = atom_positions[0];
    for &position in &atom_positions[1..] {
        minimum = minimum.min(position);
        maximum = maximum.max(position);
    }
    let maximum_radius = atom_radii.iter().copied().fold(0.0f32, f32::max);
    let padding = 2.0 * probe_radius + maximum_radius + grid_spacing;
    let origin = minimum - Vec3::splat(padding);
    let extent = maximum - minimum + Vec3::splat(2.0 * padding);
    let size = [
        (extent.x / grid_spacing).ceil() as usize,
        (extent.y / grid_spacing).ceil() as usize,
        (extent.z / grid_spacing).ceil() as usize,
    ];
    let mut matrix = Grid::filled(size, 2.0).ok_or(SurfaceError::GridAllocation(size))?;

    let grid_centers: Vec<Vec3> = atom_positions
        .iter()
        .map(|&position| (position - origin) / grid_spacing)
        .collect();
    let expanded_radii: Vec<f32> = atom_radii
        .iter()
        .map(|&radius| (radius + probe_radius) / grid_spacing)
        .collect();
    sphere_surface_distance(&grid_centers, &expanded_radii, 2.0, &mut matrix);

    let mut sas = contour_surface(&matrix, 0.0);
    if solvent_accessible {
        transform_vertices(&mut sas, origin, grid_spacing);
        return Ok(surface_geometry(sas));
    }

    matrix.fill(2.0);
    let probe_radii = vec![probe_radius / grid_spacing; sas.vertices.len()];
    sphere_surface_distance(&sas.vertices, &probe_radii, 2.0, &mut matrix);
    let mut ses = contour_surface(&matrix, 0.0);
    transform_vertices(&mut ses, origin, grid_spacing);
    let ses = remove_distant_components(ses, atom_positions, atom_radii, probe_radius);
    Ok(surface_geometry(ses))
}

fn transform_vertices(mesh: &mut ContourMesh, origin: Vec3, spacing: f32) {
    for vertex in &mut mesh.vertices {
        *vertex = origin + *vertex * spacing;
    }
}

fn surface_geometry(mesh: ContourMesh) -> SurfaceGeometry {
    SurfaceGeometry {
        vertices: mesh.vertices,
        normals: mesh.normals,
        triangles: mesh.triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_atom_ses_is_non_empty_and_finite() {
        let geometry = ses_surface_geometry(&[Vec3::ZERO], &[1.7], 1.4, 0.5, false).unwrap();
        assert!(!geometry.vertices.is_empty());
        assert!(!geometry.triangles.is_empty());
        assert!(geometry.vertices.iter().all(|v| v.is_finite()));
        assert!(geometry.normals.iter().all(|n| n.is_finite()));
    }
}
