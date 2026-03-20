use crate::Shape;
use crate::parser::mmcif::Chain;
use crate::parser::mmcif::MmCif;
use crate::parser::utils::{Residue, ResidueType::AminoAcid, SecondaryStructure};
use crate::utils::{MeshData, VisualShape, VisualStyle};
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3, Vec4};
use na_seq::AtomTypeInRes;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use wide::f32x8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub chains: Vec<Chain>,
    pub center: Vec3,

    pub style: VisualStyle,
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseMmCifError {
    #[error("Failed to parse MmCif data: '{0}'")]
    ParsingError(String),
}

impl Protein {
    pub fn from_mmcif(sdf: &str) -> Result<Self, ParseMmCifError> {
        let protein_data =
            MmCif::new(sdf).map_err(|e| ParseMmCifError::ParsingError(e.to_string()))?;
        Self::new(protein_data)
    }

    pub fn new(mmcif: MmCif) -> Result<Self, ParseMmCifError> {
        let mut chains = Vec::new();
        let mut centers = Vec::new();
        let mut residue_index = 0;

        for chain in mmcif.chains {
            let mut residues = Vec::new();

            for residue_sns in chain.residue_sns {
                let residue = &mmcif.residues[residue_index];
                residue_index += 1;
                let amino_acid = match residue.res_type.clone() {
                    AminoAcid(aa) => aa,
                    _ => continue,
                };
                let mut ca_opt = None;
                let mut c_opt = None;
                let mut n_opt = None;
                let mut o_opt = None;
                for atom_sn in &residue.atom_sns {
                    let atom = &mmcif.atoms[*atom_sn as usize - 1];
                    if let Some(atom_type_in_res) = &atom.type_in_res {
                        if *atom_type_in_res == AtomTypeInRes::C {
                            c_opt = Some(atom.posit);
                        }
                        if *atom_type_in_res == AtomTypeInRes::N {
                            n_opt = Some(atom.posit);
                        }
                        if *atom_type_in_res == AtomTypeInRes::CA {
                            ca_opt = Some(atom.posit);
                        }
                        if *atom_type_in_res == AtomTypeInRes::O {
                            o_opt = Some(atom.posit);
                        }
                    }
                }

                if ca_opt.is_none() {
                    println!(
                        "No CA atom found for chain {} residue {}",
                        chain.id, residue_sns
                    );
                    continue;
                }
                if c_opt.is_none() {
                    println!(
                        "No C atom found for chain {} residue {}",
                        chain.id, residue_sns
                    );
                    continue;
                }
                if n_opt.is_none() {
                    println!(
                        "No N atom found for chain {} residue {}",
                        chain.id, residue_sns
                    );
                    continue;
                }
                if o_opt.is_none() {
                    println!(
                        "No O atom found for chain {} residue {}",
                        chain.id, residue_sns
                    );
                    continue;
                }

                let (ca, c, n, o) = (
                    ca_opt.unwrap(),
                    c_opt.unwrap(),
                    n_opt.unwrap(),
                    o_opt.unwrap(),
                );

                centers.push(Vec3::new(ca.x as f32, ca.y as f32, ca.z as f32));

                residues.push(Residue {
                    residue_type: amino_acid,
                    ca: ca,
                    c: c,
                    n: n,
                    o: o,
                    h: None,
                    sns: residue_sns as usize,
                    ss: None,
                });
            }

            chains.push(Chain::new(chain.id.clone(), residues));
        }

        let mut center = Vec3::ZERO;
        for c in &centers {
            center += c;
        }
        center = center / (centers.len() as f32);

        Ok(Protein {
            chains: chains,
            center: center,
            style: VisualStyle {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
        })
    }
}

impl VisualShape for Protein {
    fn style_mut(&mut self) -> &mut VisualStyle {
        &mut self.style
    }
}

impl Protein {
    pub fn get_center(&self) -> [f32; 3] {
        [self.center.x, self.center.y, self.center.z]
    }

    pub fn centered(mut self) -> Self {
        let center = Vec3 {
            x: self.center.x,
            y: self.center.y,
            z: self.center.z,
        };
        for chain in &mut self.chains {
            for residue in &mut chain.residues {
                residue.ca -= center;
                residue.c -= center;
                residue.n -= center;
                residue.o -= center;
                if let Some(h) = residue.h {
                    residue.h = Some(h - center);
                }
            }
        }
        self.center = Vec3::ZERO;
        self
    }

    fn catmull_rom_chain(&self, positions: &[Vec3], pts_per_res: usize) -> Vec<Vec3> {
        let n = positions.len();
        if n < 2 {
            return positions.to_vec();
        }

        // 精确预分配（关键！）
        let total_points = 1 + (n - 1) * pts_per_res;
        let mut path = Vec::with_capacity(total_points);
        path.push(positions[0]);

        for i in 0..n - 1 {
            let p0 = if i > 0 {
                positions[i - 1]
            } else {
                positions[0]
            };
            let p1 = positions[i];
            let p2 = positions[i + 1];
            let p3 = if i + 2 < n {
                positions[i + 2]
            } else {
                positions[i + 1]
            };

            // 预计算步长
            let step = 1.0 / pts_per_res as f32;
            let mut t = step;

            for _ in 1..=pts_per_res {
                path.push(catmull_rom(p0, p1, p2, p3, t));
                t += step;
            }
        }
        path
    }

    pub fn to_mesh(&self, scale: f32) -> MeshData {
        use std::time::Instant;
        let start_total = Instant::now();
        let pts_per_res = 5;

        // println!("to_mesh started");

        let mut final_mesh = MeshData::default();

        // let total_res: usize = self.chains.iter().map(|c| c.residues.len()).sum();
        // let estimated_verts = total_res * 8 * pts_per_res; // 粗估
        // final_mesh.vertices.reserve(estimated_verts);
        // final_mesh.normals.reserve(estimated_verts);
        // final_mesh.indices.reserve(estimated_verts * 2);

        // println!("reserve{} {}", estimated_verts, estimated_verts * 2);

        self.chains.par_iter().for_each(|chain| {
            chain.get_ss();
        });

        let meshes: Vec<MeshData> = self
            .chains
            // .iter()
            .par_iter()
            .filter_map(|chain| {
                let mut mesh = MeshData::default();

                // === residues ===
                let residues: Vec<&Residue> = chain
                    .residues
                    .iter()
                    .filter(|r| r.ca.length_squared() > 1e-6)
                    .collect();

                if residues.len() < 2 {
                    return None;
                }

                let ca_positions: Vec<Vec3> = residues.iter().map(|r| r.ca).collect();

                let path = self.catmull_rom_chain(&ca_positions, pts_per_res);
                let n = path.len();

                let mut centers = Vec::with_capacity(n);
                let mut tangents = Vec::with_capacity(n);
                let mut normals = Vec::with_capacity(n);

                // === tangent ===
                for i in 0..n {
                    centers.push(path[i]);

                    let p0 = if i > 0 { path[i - 1] } else { path[0] };
                    let p1 = path[i];
                    let p2 = if i + 1 < n { path[i + 1] } else { path[i] };
                    let p3 = if i + 2 < n { path[i + 2] } else { p2 };

                    tangents.push(catmull_rom_tangent(p0, p1, p2, p3).normalize_or_zero());
                }

                // === normal (parallel transport) ===
                fn initial_normal(t: Vec3) -> Vec3 {
                    if t.dot(Vec3::Z).abs() < 0.98 {
                        t.cross(Vec3::Z).normalize()
                    } else {
                        t.cross(Vec3::X).normalize()
                    }
                }

                let mut current_normal = initial_normal(tangents[0]);
                normals.push(current_normal);

                for i in 1..n {
                    let prev_t = tangents[i - 1];
                    let curr_t = tangents[i];

                    let axis = prev_t.cross(curr_t);
                    if axis.length_squared() > 1e-6 {
                        let angle = prev_t.angle_between(curr_t);
                        let q = Quat::from_axis_angle(axis.normalize(), angle);
                        current_normal = q * current_normal;
                    }
                    normals.push(current_normal);
                }

                // === secondary structure ===
                let ss = chain.get_ss();

                // === 🔥 核心：per-residue extrusion ===
                for (res_idx, ss_type) in ss.iter().enumerate() {
                    // 对应 spline 区间
                    let i0 = res_idx * pts_per_res;
                    let i1 = ((res_idx + 1) * pts_per_res + 1).min(n);

                    let mid = (i0 + i1) / 2;

                    // === ChimeraX: front / back section ===
                    let section_front = match ss_type {
                        SecondaryStructure::Helix => &*HELIX_SECTION,
                        SecondaryStructure::Sheet => &*SHEET_SECTION,
                        _ => &*COIL_SECTION,
                    };

                    // ⚠️ 这里先简化：front/back 相同
                    // 后续可以做 arrow / transition
                    let section_back = section_front;

                    if res_idx == ss.len() - 1 {
                        continue;
                    }

                    // println!("{:?} {:?}", [i0..mid], [mid..i1]);

                    // === front half ===
                    {
                        let sub_centers = &centers[i0..mid + 1];
                        let sub_tangents = &tangents[i0..mid + 1];
                        let sub_normals = &normals[i0..mid + 1];

                        if sub_centers.len() > 1 {
                            let scales = vec![1.0; sub_centers.len()];
                            let sub_mesh = section_front.extrude(
                                sub_centers,
                                sub_tangents,
                                sub_normals,
                                &scales,
                            );
                            mesh.append(&sub_mesh);
                        }
                    }

                    // === back half ===
                    {
                        let sub_centers = &centers[mid..i1];
                        let sub_tangents = &tangents[mid..i1];
                        let sub_normals = &normals[mid..i1];

                        if sub_centers.len() > 1 {
                            let scales = vec![1.0; sub_centers.len()];
                            let sub_mesh = section_back.extrude(
                                sub_centers,
                                sub_tangents,
                                sub_normals,
                                &scales,
                            );
                            mesh.append(&sub_mesh);
                        }
                    }

                    // add flat cap
                    if res_idx == 0 {
                        let cap = section_front.add_flat_cap(
                            &centers[i0],
                            &tangents[i0],
                            &normals[i0],
                            1.0,
                        );
                        mesh.append(&cap);
                    } else if !matches!(
                        (&ss[res_idx - 1], ss_type),
                        (SecondaryStructure::Helix, SecondaryStructure::Helix)
                            | (SecondaryStructure::Sheet, SecondaryStructure::Sheet)
                            | (
                                SecondaryStructure::Coil | SecondaryStructure::Turn,
                                SecondaryStructure::Coil | SecondaryStructure::Turn
                            )
                    ) {
                        // 处理 turn -> coil
                        let (section, is_first) = cap_use(&ss[res_idx - 1], ss_type);
                        if is_first {
                            let cap = section.add_flat_cap(
                                &centers[i0],
                                &-tangents[i0],
                                &normals[i0],
                                1.0,
                            );
                            mesh.append(&cap);
                        } else {
                            let cap = section.add_flat_cap(
                                &centers[i0],
                                &tangents[i0],
                                &normals[i0],
                                1.0,
                            );
                            mesh.append(&cap);
                        }
                    }
                    // add caps for the last residue
                    if res_idx == ss.len() - 2 {
                        let cap = section_back.add_flat_cap(
                            &centers[i1 - 1],
                            &-tangents[i1 - 1],
                            &normals[i1 - 1],
                            1.0,
                        );
                        mesh.append(&cap);
                    }
                }

                // === scale ===
                for v in &mut mesh.vertices {
                    *v *= scale;
                }

                // === color ===
                let color = match self.style.color {
                    Some(color) => Vec4::new(color[0], color[1], color[2], self.style.opacity),
                    None => Vec4::new(1.0, 1.0, 1.0, 1.0),
                };

                mesh.colors = Some(vec![color; mesh.vertices.len()]);

                Some(mesh)
            })
            .collect();

        for mesh in meshes {
            final_mesh.append(&mesh);
        }
        // println!(
        //     "chain {} processed in {:?}",
        //     chain.id,
        //     start_chain.elapsed()
        // );
        // }

        // println!(
        //     "actual length {} {}",
        //     final_mesh.vertices.len(),
        //     final_mesh.indices.len()
        // );

        println!("to_mesh finished in {:?}", start_total.elapsed());
        final_mesh
    }
}

// 加这个函数
#[inline(always)]
fn catmull_rom_tangent(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3) -> Vec3 {
    // 标准公式：(p2 - p0) + 0.5 * (p3 - p1)
    let a = p2 - p0;
    let b = p3 - p1;
    (a + b * 0.5).normalize_or_zero()
}

// 标准 Catmull-Rom 公式（ChimeraX、Mol*、PyMOL、VMD 全都用这个）
#[inline(always)]
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;

    // Catmull-Rom 系数（tension = 0.5）
    let c0 = -0.5 * t3 + t2 - 0.5 * t;
    let c1 = 1.5 * t3 - 2.5 * t2 + 1.0;
    let c2 = -1.5 * t3 + 2.0 * t2 + 0.5 * t;
    let c3 = 0.5 * t3 - 0.5 * t2;

    p0 * c0 + p1 * c1 + p2 * c2 + p3 * c3
}

impl Into<Shape> for Protein {
    fn into(self) -> Shape {
        Shape::Protein(self)
    }
}

use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct RibbonXSection {
    // 2D 截面
    pub coords: Vec<[f32; 2]>,
    pub coords2: Option<Vec<[f32; 2]>>, // arrow 后半段

    // 法线（2D）
    pub normals: Vec<[f32; 2]>,
    pub normals2: Option<Vec<[f32; 2]>>,

    pub is_arrow: bool,
    pub is_faceted: bool,

    // 三角化（index）
    pub tessellation: Vec<[u32; 3]>,
}

impl RibbonXSection {
    pub fn new(
        coords: Vec<[f32; 2]>,
        coords2: Option<Vec<[f32; 2]>>,
        normals: Option<Vec<[f32; 2]>>,
        normals2: Option<Vec<[f32; 2]>>,
        faceted: bool,
        tessellation: Option<Vec<[u32; 3]>>,
    ) -> Self {
        if coords.is_empty() {
            panic!("no ribbon cross section coordinates");
        }

        let is_arrow = coords2.is_some();

        let (normals, normals2, is_faceted) = match (normals, normals2) {
            (None, _) => {
                let generated = Self::generate_normals(&coords);
                (generated, None, faceted)
            }
            (Some(n), None) => (Self::normalize_normals(n), None, false),
            (Some(n1), Some(n2)) => (
                Self::normalize_normals(n1),
                Some(Self::normalize_normals(n2)),
                true,
            ),
        };

        let tessellation = tessellation.unwrap_or_else(|| Self::tessellate(&coords));

        Self {
            coords,
            coords2,
            normals,
            normals2,
            is_arrow,
            is_faceted,
            tessellation,
        }
    }
    fn generate_normals(coords: &[[f32; 2]]) -> Vec<[f32; 2]> {
        let n = coords.len();
        let mut normals = vec![[0.0, 0.0]; n];

        for i in 0..n {
            let p0 = coords[i];
            let p1 = coords[(i + 1) % n];

            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];

            // 垂直方向（2D法线）
            let normal = [-dy, dx];

            normals[i] = Self::normalize2(normal);
        }

        normals
    }

    fn normalize_normals(mut normals: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
        for n in &mut normals {
            *n = Self::normalize2(*n);
        }
        normals
    }

    fn normalize2(v: [f32; 2]) -> [f32; 2] {
        let len = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-6);
        [v[0] / len, v[1] / len]
    }
    fn tessellate(coords: &[[f32; 2]]) -> Vec<[u32; 3]> {
        // 假设 convex polygon（够用）
        let mut tris = Vec::new();

        for i in 1..coords.len() - 1 {
            tris.push([0, i as u32, (i + 1) as u32]);
        }

        tris
    }
    pub fn extrude(
        &self,
        centers: &[Vec3],
        tangents: &[Vec3],
        normals_3d: &[Vec3],
        scales: &[f32],
    ) -> MeshData {
        if self.is_faceted {
            unreachable!()
            // self.extrude_faceted(centers, tangents, normals_3d, scales)
        } else {
            self.extrude_smooth(centers, tangents, normals_3d, scales)
        }
    }
    fn extrude_smooth(
        &self,
        centers: &[Vec3],
        tangents: &[Vec3],
        normals_3d: &[Vec3],
        scales: &[f32],
    ) -> MeshData {
        let mut mesh = MeshData {
            vertices: vec![],
            colors: None,
            normals: vec![],
            indices: vec![],
            transform: None,
            is_wireframe: false,
        };

        let n_section = self.coords.len();

        for i in 0..centers.len() {
            let c = centers[i];
            let t = tangents[i];
            let n = normals_3d[i];
            let b = t.cross(n);

            let scale = scales[i];

            for j in 0..n_section {
                let [x, y] = self.coords[j];

                let pos = c + (n * x * scale + b * y * scale);

                let [nx, ny] = self.normals[j];
                let normal = ((n * nx) + (b * ny)).normalize();

                mesh.vertices.push(pos);
                mesh.normals.push(normal);
            }
        }

        // 连接 strip
        for i in 0..centers.len() - 1 {
            let base0 = i * n_section;
            let base1 = (i + 1) * n_section;

            for j in 0..n_section {
                let j_next = (j + 1) % n_section;

                mesh.indices.extend_from_slice(&[
                    (base1 + j) as u32,
                    (base0 + j) as u32,
                    (base1 + j_next) as u32,
                    (base1 + j_next) as u32,
                    (base0 + j) as u32,
                    (base0 + j_next) as u32,
                ]);
            }
        }

        mesh
    }

    fn add_flat_cap(&self, center: &Vec3, tangent: &Vec3, normal: &Vec3, scale: f32) -> MeshData {
        let mut mesh = MeshData {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            colors: None,
            transform: None,
            is_wireframe: false,
        };
        let c = center;
        let t = tangent;
        let n = normal;
        let b = t.cross(*n);

        let n_section = self.coords.len();

        mesh.vertices.push(*c);
        mesh.normals.push(*n);

        for j in 0..n_section {
            let [x, y] = self.coords[j];
            let pos = c + (n * x * scale + b * y * scale);
            mesh.vertices.push(pos);
            mesh.normals.push(*normal);
        }

        for j in 0..n_section {
            mesh.indices
                .extend_from_slice(&[0, ((j + 1) % n_section) as u32 + 1, j as u32 + 1]);
        }

        mesh
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SimdVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

fn make_round_section(radius_x: f32, radius_y: f32, n: usize) -> RibbonXSection {
    let mut coords = Vec::with_capacity(n);
    let mut normals = Vec::with_capacity(n);

    for i in 0..n {
        let theta = (i as f32 / n as f32) * std::f32::consts::TAU;

        let x = theta.cos();
        let y = theta.sin();

        coords.push([x * radius_x, y * radius_y]);

        // normal = outward
        normals.push([x, y]);
    }

    RibbonXSection::new(coords, None, Some(normals), None, false, None)
}
fn make_square_section(width: f32, height: f32) -> RibbonXSection {
    let hw = width * 0.5;
    let hh = height * 0.5;

    let coords = vec![
        [-hw, -hh],
        [-hw, -hh],
        [hw, -hh],
        [hw, -hh],
        [hw, hh],
        [hw, hh],
        [-hw, hh],
        [-hw, hh],
    ];

    // 法线（面向外）
    let normals = vec![
        [-1.0, 0.0],
        [0.0, -1.0],
        [0.0, -1.0],
        [1.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [0.0, 1.0],
        [-1.0, 0.0],
    ];

    RibbonXSection::new(coords, None, Some(normals), None, true, None)
}

pub static HELIX_SECTION: Lazy<RibbonXSection> = Lazy::new(|| {
    // ChimeraX: helix = 圆柱，略扁
    make_round_section(1.0, 1.0, 16)
});

pub static COIL_SECTION: Lazy<RibbonXSection> = Lazy::new(|| {
    // 更细
    make_round_section(0.25, 0.25, 12)
});

pub static SHEET_SECTION: Lazy<RibbonXSection> = Lazy::new(|| {
    // 扁带（宽 >> 厚）
    make_square_section(2.0, 0.3)
});

fn cap_use<'a>(
    ss_a: &'a SecondaryStructure,
    ss_b: &'a SecondaryStructure,
) -> (&'a RibbonXSection, bool) {
    fn priority(ss: &SecondaryStructure) -> u8 {
        match ss {
            SecondaryStructure::Sheet => 3,
            SecondaryStructure::Helix => 2,
            SecondaryStructure::Coil | SecondaryStructure::Turn => 1,
        }
    }

    let pa = priority(ss_a);
    let pb = priority(ss_b);

    let (chosen, idx) = if pa >= pb {
        (ss_a, true)
    } else {
        (ss_b, false)
    };

    let section = match chosen {
        SecondaryStructure::Sheet => &*SHEET_SECTION,
        SecondaryStructure::Helix => &*HELIX_SECTION,
        SecondaryStructure::Coil | SecondaryStructure::Turn => &*COIL_SECTION,
    };

    (section, idx)
}
