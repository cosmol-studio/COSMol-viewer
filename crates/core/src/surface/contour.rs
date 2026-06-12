// Derived from UCSF ChimeraX map/_map/contour.cpp.
// Copyright 2022 Regents of the University of California.
// Licensed under GNU LGPL version 2.1; see surface/NOTICE.

use glam::Vec3;

use super::contourdata::TRIANGLE_TABLE;
use super::distance_grid::Grid;

const NO_VERTEX: u32 = u32::MAX;

const EDGE_A00: usize = 0;
const EDGE_1A0: usize = 1;
const EDGE_A10: usize = 2;
const EDGE_0A0: usize = 3;
const EDGE_A01: usize = 4;
const EDGE_1A1: usize = 5;
const EDGE_A11: usize = 6;
const EDGE_0A1: usize = 7;
const EDGE_00A: usize = 8;
const EDGE_10A: usize = 9;
const EDGE_11A: usize = 10;
const EDGE_01A: usize = 11;

#[derive(Clone)]
struct GridCell {
    k0: usize,
    k1: usize,
    vertex: [u32; 20],
}

struct GridCellList {
    size0: usize,
    size1: usize,
    table: Vec<Option<usize>>,
    cells: Vec<GridCell>,
}

impl GridCellList {
    fn new(size0: usize, size1: usize) -> Self {
        Self {
            size0,
            size1,
            table: vec![None; size0 * size1],
            cells: Vec::new(),
        }
    }

    fn cell(&mut self, k0: isize, k1: isize) -> Option<&mut GridCell> {
        if k0 < 0 || k1 < 0 || k0 as usize >= self.size0 || k1 as usize >= self.size1 {
            return None;
        }
        let table_index = k1 as usize * self.size0 + k0 as usize;
        let cell_index = match self.table[table_index] {
            Some(index) => index,
            None => {
                let index = self.cells.len();
                self.cells.push(GridCell {
                    k0: k0 as usize,
                    k1: k1 as usize,
                    vertex: [NO_VERTEX; 20],
                });
                self.table[table_index] = Some(index);
                index
            }
        };
        Some(&mut self.cells[cell_index])
    }

    fn set_edge_vertex(&mut self, k0: isize, k1: isize, edge: usize, vertex: u32) {
        if let Some(cell) = self.cell(k0, k1) {
            cell.vertex[edge] = vertex;
        }
    }

    fn finished_plane(&mut self) {
        self.cells.clear();
        self.table.fill(None);
    }
}

pub(crate) struct ContourMesh {
    pub(crate) vertices: Vec<Vec3>,
    pub(crate) normals: Vec<Vec3>,
    pub(crate) triangles: Vec<[u32; 3]>,
}

pub(crate) fn contour_surface(grid: &Grid, threshold: f32) -> ContourMesh {
    let mut contour = Contour::new(grid, threshold);
    contour.compute_surface();
    let normals = contour.normals();
    ContourMesh {
        vertices: contour.vertices,
        normals,
        triangles: contour.triangles,
    }
}

struct Contour<'a> {
    grid: &'a Grid,
    threshold: f32,
    vertices: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
}

impl<'a> Contour<'a> {
    fn new(grid: &'a Grid, threshold: f32) -> Self {
        Self {
            grid,
            threshold,
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
    }

    fn compute_surface(&mut self) {
        if self.grid.size.iter().any(|&size| size < 2) {
            return;
        }

        let mut planes = [
            GridCellList::new(self.grid.size[0] - 1, self.grid.size[1] - 1),
            GridCellList::new(self.grid.size[0] - 1, self.grid.size[1] - 1),
        ];

        for k2 in 0..self.grid.size[2] {
            let (gp0, gp1) = if k2 % 2 == 0 {
                let (left, right) = planes.split_at_mut(1);
                (&mut left[0], &mut right[0])
            } else {
                let (left, right) = planes.split_at_mut(1);
                (&mut right[0], &mut left[0])
            };
            self.mark_plane_edge_cuts(gp0, gp1, k2);
            if k2 > 0 {
                self.make_triangles(gp0, k2);
            }
            gp0.finished_plane();
        }
    }

    fn mark_plane_edge_cuts(&mut self, gp0: &mut GridCellList, gp1: &mut GridCellList, k2: usize) {
        let [k0_size, k1_size, k2_size] = self.grid.size;
        for k1 in 0..k1_size {
            if k1 == 0 || k1 + 1 == k1_size || k2 == 0 || k2 + 1 == k2_size {
                for k0 in 0..k0_size {
                    self.mark_boundary_edge_cuts(k0, k1, k2, gp0, gp1);
                }
            } else {
                if k0_size > 0 {
                    self.mark_boundary_edge_cuts(0, k1, k2, gp0, gp1);
                }
                self.mark_interior_edge_cuts(k1, k2, gp0, gp1);
                if k0_size > 1 {
                    self.mark_boundary_edge_cuts(k0_size - 1, k1, k2, gp0, gp1);
                }
            }
        }
    }

    fn mark_interior_edge_cuts(
        &mut self,
        k1: usize,
        k2: usize,
        gp0: &mut GridCellList,
        gp1: &mut GridCellList,
    ) {
        let k0_max = self.grid.size[0].saturating_sub(1);
        for k0 in 1..k0_max {
            let v0 = self.grid.value(k0, k1, k2) - self.threshold;
            if v0 < 0.0 {
                continue;
            }

            let mut v1 = self.grid.value(k0 - 1, k1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_0(k0 - 1, k1, k2, k0 as f32 - v0 / (v0 - v1), gp0, gp1);
            }
            v1 = self.grid.value(k0 + 1, k1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_0(k0, k1, k2, k0 as f32 + v0 / (v0 - v1), gp0, gp1);
            }
            v1 = self.grid.value(k0, k1 - 1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_1(k0, k1 - 1, k2, k1 as f32 - v0 / (v0 - v1), gp0, gp1);
            }
            v1 = self.grid.value(k0, k1 + 1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_1(k0, k1, k2, k1 as f32 + v0 / (v0 - v1), gp0, gp1);
            }
            v1 = self.grid.value(k0, k1, k2 - 1) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_2(k0, k1, k2 as f32 - v0 / (v0 - v1), gp0);
            }
            v1 = self.grid.value(k0, k1, k2 + 1) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_2(k0, k1, k2 as f32 + v0 / (v0 - v1), gp1);
            }
        }
    }

    fn mark_boundary_edge_cuts(
        &mut self,
        k0: usize,
        k1: usize,
        k2: usize,
        gp0: &mut GridCellList,
        gp1: &mut GridCellList,
    ) {
        let [k0_size, k1_size, k2_size] = self.grid.size;
        let v0 = self.grid.value(k0, k1, k2) - self.threshold;
        if v0 < 0.0 {
            return;
        }

        if k0 > 0 {
            let v1 = self.grid.value(k0 - 1, k1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_0(k0 - 1, k1, k2, k0 as f32 - v0 / (v0 - v1), gp0, gp1);
            }
        }
        if k0 + 1 < k0_size {
            let v1 = self.grid.value(k0 + 1, k1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_0(k0, k1, k2, k0 as f32 + v0 / (v0 - v1), gp0, gp1);
            }
        }
        if k1 > 0 {
            let v1 = self.grid.value(k0, k1 - 1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_1(k0, k1 - 1, k2, k1 as f32 - v0 / (v0 - v1), gp0, gp1);
            }
        }
        if k1 + 1 < k1_size {
            let v1 = self.grid.value(k0, k1 + 1, k2) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_1(k0, k1, k2, k1 as f32 + v0 / (v0 - v1), gp0, gp1);
            }
        }
        if k2 > 0 {
            let v1 = self.grid.value(k0, k1, k2 - 1) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_2(k0, k1, k2 as f32 - v0 / (v0 - v1), gp0);
            }
        }
        if k2 + 1 < k2_size {
            let v1 = self.grid.value(k0, k1, k2 + 1) - self.threshold;
            if v1 < 0.0 {
                self.add_vertex_axis_2(k0, k1, k2 as f32 + v0 / (v0 - v1), gp1);
            }
        }
    }

    fn add_vertex_axis_0(
        &mut self,
        k0: usize,
        k1: usize,
        k2: usize,
        x0: f32,
        gp0: &mut GridCellList,
        gp1: &mut GridCellList,
    ) {
        let vertex = self.create_vertex(x0, k1 as f32, k2 as f32);
        gp0.set_edge_vertex(k0 as isize, k1 as isize - 1, EDGE_A11, vertex);
        gp0.set_edge_vertex(k0 as isize, k1 as isize, EDGE_A01, vertex);
        gp1.set_edge_vertex(k0 as isize, k1 as isize - 1, EDGE_A10, vertex);
        gp1.set_edge_vertex(k0 as isize, k1 as isize, EDGE_A00, vertex);
    }

    fn add_vertex_axis_1(
        &mut self,
        k0: usize,
        k1: usize,
        k2: usize,
        x1: f32,
        gp0: &mut GridCellList,
        gp1: &mut GridCellList,
    ) {
        let vertex = self.create_vertex(k0 as f32, x1, k2 as f32);
        gp0.set_edge_vertex(k0 as isize - 1, k1 as isize, EDGE_1A1, vertex);
        gp0.set_edge_vertex(k0 as isize, k1 as isize, EDGE_0A1, vertex);
        gp1.set_edge_vertex(k0 as isize - 1, k1 as isize, EDGE_1A0, vertex);
        gp1.set_edge_vertex(k0 as isize, k1 as isize, EDGE_0A0, vertex);
    }

    fn add_vertex_axis_2(&mut self, k0: usize, k1: usize, x2: f32, gp: &mut GridCellList) {
        let vertex = self.create_vertex(k0 as f32, k1 as f32, x2);
        gp.set_edge_vertex(k0 as isize, k1 as isize, EDGE_00A, vertex);
        gp.set_edge_vertex(k0 as isize - 1, k1 as isize, EDGE_10A, vertex);
        gp.set_edge_vertex(k0 as isize, k1 as isize - 1, EDGE_01A, vertex);
        gp.set_edge_vertex(k0 as isize - 1, k1 as isize - 1, EDGE_11A, vertex);
    }

    fn create_vertex(&mut self, x: f32, y: f32, z: f32) -> u32 {
        let index = self.vertices.len() as u32;
        self.vertices.push(Vec3::new(x, y, z));
        index
    }

    fn make_triangles(&mut self, plane: &GridCellList, k2: usize) {
        for cell in &plane.cells {
            let k0 = cell.k0;
            let k1 = cell.k1;
            let bits = (self.grid.value(k0, k1, k2 - 1) >= self.threshold) as usize
                | (((self.grid.value(k0 + 1, k1, k2 - 1) >= self.threshold) as usize) << 1)
                | (((self.grid.value(k0 + 1, k1 + 1, k2 - 1) >= self.threshold) as usize) << 2)
                | (((self.grid.value(k0, k1 + 1, k2 - 1) >= self.threshold) as usize) << 3)
                | (((self.grid.value(k0, k1, k2) >= self.threshold) as usize) << 4)
                | (((self.grid.value(k0 + 1, k1, k2) >= self.threshold) as usize) << 5)
                | (((self.grid.value(k0 + 1, k1 + 1, k2) >= self.threshold) as usize) << 6)
                | (((self.grid.value(k0, k1 + 1, k2) >= self.threshold) as usize) << 7);

            for triangle in TRIANGLE_TABLE[bits]
                .split(|&edge| edge == -1)
                .next()
                .unwrap_or_default()
                .chunks_exact(3)
            {
                let indices = [
                    cell.vertex[triangle[0] as usize],
                    cell.vertex[triangle[1] as usize],
                    cell.vertex[triangle[2] as usize],
                ];
                debug_assert!(indices.iter().all(|&index| index != NO_VERTEX));
                if indices.iter().all(|&index| index != NO_VERTEX) {
                    self.triangles.push(indices);
                }
            }
        }
    }

    fn normals(&self) -> Vec<Vec3> {
        self.vertices
            .iter()
            .map(|&position| {
                let x = position.to_array();
                let mut gradient = [0.0f32; 3];
                for axis in 0..3 {
                    gradient[axis] = if x[axis] == 0.0 {
                        1.0
                    } else if x[axis] == (self.grid.size[axis] - 1) as f32 {
                        -1.0
                    } else {
                        0.0
                    };
                }

                if gradient == [0.0; 3] {
                    let i = [x[0] as usize, x[1] as usize, x[2] as usize];
                    let ga = i[0] * self.grid.stride[0]
                        + i[1] * self.grid.stride[1]
                        + i[2] * self.grid.stride[2];
                    let mut gb = ga;
                    let mut offset = [0usize; 3];
                    let mut fb = 0.0;
                    for axis in 0..3 {
                        fb = x[axis] - i[axis] as f32;
                        if fb > 0.0 {
                            offset[axis] = 1;
                            gb = ga + self.grid.stride[axis];
                            break;
                        }
                    }
                    let fa = 1.0 - fb;
                    for axis in 0..3 {
                        let stride = self.grid.stride[axis];
                        let ia = i[axis];
                        let ib = ia + offset[axis];
                        let da = if ia == 0 {
                            2.0 * (self.grid.data[ga + stride] - self.grid.data[ga])
                        } else {
                            self.grid.data[ga + stride] - self.grid.data[ga - stride]
                        };
                        let db = if ib == 0 {
                            2.0 * (self.grid.data[gb + stride] - self.grid.data[gb])
                        } else if ib == self.grid.size[axis] - 1 {
                            2.0 * (self.grid.data[gb] - self.grid.data[gb - stride])
                        } else {
                            self.grid.data[gb + stride] - self.grid.data[gb - stride]
                        };
                        gradient[axis] = fa * da + fb * db;
                    }
                    let length = Vec3::from_array(gradient).length();
                    if length > 0.0 {
                        gradient[0] /= length;
                        gradient[1] /= length;
                        gradient[2] /= length;
                    }
                }
                -Vec3::from_array(gradient)
            })
            .collect()
    }
}
