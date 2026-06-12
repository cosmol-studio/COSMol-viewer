// Derived from UCSF ChimeraX map/_map/distgrid.cpp.
// Copyright 2022 Regents of the University of California.
// Licensed under GNU LGPL version 2.1; see surface/NOTICE.

use glam::Vec3;

pub(crate) struct Grid {
    pub(crate) data: Vec<f32>,
    pub(crate) size: [usize; 3],
    pub(crate) stride: [usize; 3],
}

impl Grid {
    pub(crate) fn filled(size: [usize; 3], value: f32) -> Option<Self> {
        let len = size[0].checked_mul(size[1])?.checked_mul(size[2])?;
        let mut data = Vec::new();
        data.try_reserve_exact(len).ok()?;
        data.resize(len, value);
        Some(Self {
            data,
            size,
            stride: [1, size[0], size[0] * size[1]],
        })
    }

    #[inline]
    pub(crate) fn value(&self, i: usize, j: usize, k: usize) -> f32 {
        self.data[k * self.stride[2] + j * self.stride[1] + i]
    }

    pub(crate) fn fill(&mut self, value: f32) {
        self.data.fill(value);
    }
}

pub(crate) fn sphere_surface_distance(
    centers: &[Vec3],
    radii: &[f32],
    max_range: f32,
    matrix: &mut Grid,
) {
    debug_assert_eq!(centers.len(), radii.len());

    for (&center, &radius) in centers.iter().zip(radii) {
        if radius == 0.0 {
            continue;
        }

        let cijk = center.to_array();
        let mut ijk_min = [0usize; 3];
        let mut ijk_max = [0usize; 3];
        for axis in 0..3 {
            ijk_min[axis] = clamp(
                (cijk[axis] - (radius + max_range)).ceil() as isize,
                matrix.size[axis],
            );
            ijk_max[axis] = clamp(
                (cijk[axis] + (radius + max_range)).floor() as isize,
                matrix.size[axis],
            );
        }

        for k in ijk_min[2]..=ijk_max[2] {
            let dk = k as f32 - cijk[2];
            let k2 = dk * dk;
            for j in ijk_min[1]..=ijk_max[1] {
                let dj = j as f32 - cijk[1];
                let jk2 = dj * dj + k2;
                for i in ijk_min[0]..=ijk_max[0] {
                    let di = i as f32 - cijk[0];
                    let rd = (di * di + jk2).sqrt() - radius;
                    let index = k * matrix.stride[2] + j * matrix.stride[1] + i * matrix.stride[0];
                    if rd < matrix.data[index] {
                        matrix.data[index] = rd;
                    }
                }
            }
        }
    }
}

#[inline]
fn clamp(value: isize, limit: usize) -> usize {
    if value < 0 {
        0
    } else if value as usize >= limit {
        limit - 1
    } else {
        value as usize
    }
}
