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
        // use std::time::Instant;
        // let start_total = Instant::now();
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
            .par_iter()
            .filter_map(|chain| {
                // let start_chain = Instant::now();

                let mut mesh = MeshData::default();

                // 筛选有效残基
                let residues: Vec<&Residue> = chain
                    .residues
                    .iter()
                    .filter(|r| r.ca.length_squared() > 1e-6)
                    .collect();

                let ca_positions: Vec<Vec3> = residues.iter().map(|r| r.ca).collect();

                if ca_positions.len() < 2 {
                    println!("chain {} has less than 2 residues, skipping", chain.id);
                    return None;
                }

                // 生成平滑路径
                let path = self.catmull_rom_chain(&ca_positions, pts_per_res);

                let n = path.len();
                let mut centers = Vec::with_capacity(n);
                let mut tangents = Vec::with_capacity(n);
                let mut normals = Vec::with_capacity(n);

                // === 计算 centers + tangents ===
                // let start_tangent = Instant::now();
                for i in 0..n {
                    centers.push(path[i]);
                    let p0 = if i > 0 { path[i - 1] } else { path[0] };
                    let p1 = path[i];
                    let p2 = if i + 1 < n { path[i + 1] } else { path[i] };
                    let p3 = if i + 2 < n { path[i + 2] } else { p2 };
                    tangents.push(catmull_rom_tangent(p0, p1, p2, p3).normalize_or_zero());
                }
                // println!("  tangent calculation: {:?}", start_tangent.elapsed());

                // === 初始法线 + Parallel Transport Frame ===
                // let start_normal = Instant::now();
                fn initial_normal(t: Vec3) -> Vec3 {
                    if t.dot(Vec3::Z).abs() < 0.98 {
                        t.cross(Vec3::Z).normalize()
                    } else {
                        t.cross(Vec3::X).normalize()
                    }
                }

                let mut current_normal = initial_normal(tangents[0]);
                normals.push(current_normal);

                for i in 1..centers.len() {
                    let prev_t = tangents[i - 1];
                    let curr_t = tangents[i];

                    let rotation_axis = prev_t.cross(curr_t);
                    if rotation_axis.length_squared() > 1e-6 {
                        let rotation_angle = prev_t.angle_between(curr_t);
                        let rotation =
                            Quat::from_axis_angle(rotation_axis.normalize(), rotation_angle);
                        current_normal = rotation * current_normal;
                    }
                    normals.push(current_normal);
                }

                // println!("  normal calculation: {:?}", start_normal.elapsed());

                // === sections ===
                // let start_section = Instant::now();
                let sections: Vec<&RibbonXSection> = chain
                    .get_ss()
                    .iter()
                    .map(|r| match r {
                        SecondaryStructure::Helix => &*HELIX_SECTION,
                        SecondaryStructure::Sheet => &*SHEET_SECTION,
                        _ => &*COIL_SECTION,
                    })
                    .collect();
                // println!("  section lookup: {:?}", start_section.elapsed());

                // === extrusion ===
                // let start_extrude = Instant::now();
                self.extrude_ribbon_corrected(
                    &centers,
                    &tangents,
                    &normals,
                    &sections,
                    pts_per_res,
                    &mut mesh,
                );
                // println!("  extrusion: {:?}", start_extrude.elapsed());

                // === scale + colors ===
                // let start_post = Instant::now();
                for v in &mut mesh.vertices {
                    *v *= scale;
                }

                let color = match self.style.color {
                    Some(color) => Vec4::new(color[0], color[1], color[2], self.style.opacity),
                    None => Vec4::new(1.0, 1.0, 1.0, 1.0),
                };

                mesh.colors = Some(vec![color; mesh.vertices.len()]);
                // println!("  postprocess: {:?}", start_post.elapsed());

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

        // println!("to_mesh finished in {:?}", start_total.elapsed());
        final_mesh
    }

    // 完全修正版的 extrusion（不再有任何越界、箭头方向、端盖问题）
    fn extrude_ribbon_corrected(
        &self,
        centers: &[Vec3],
        tangents: &[Vec3],
        normals: &[Vec3],
        sections: &[&RibbonXSection],
        pts_per_res: usize,
        mesh: &mut MeshData,
    ) {
        let base_v = mesh.vertices.len() as u32;

        for (seg, xs) in sections.iter().enumerate() {
            let start = seg * pts_per_res;
            let end = if seg + 1 < sections.len() {
                (seg + 1) * pts_per_res + 1
            } else {
                centers.len()
            };

            let cap_front = seg == 0;
            let cap_back = seg + 1 == sections.len();

            let (coords, arrow_back) = if xs.arrow_coords.is_some() {
                (&xs.coords[..], xs.arrow_coords.as_deref())
            } else {
                (&xs.coords[..], None)
            };

            self.extrude_one_segment_simd(
                &centers[start..end],
                &tangents[start..end],
                &normals[start..end],
                coords,
                arrow_back,
                cap_front,
                cap_back,
                mesh,
                xs.ss,
            );
        }

        // 统一偏移索引
        for idx in &mut mesh.indices {
            *idx += base_v;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extrude_one_segment_simd(
        &self,
        centers: &[Vec3],
        tangents: &[Vec3],
        normals: &[Vec3],
        coords: &[[f32; 2]],
        arrow_back: Option<&[[f32; 2]]>,
        cap_front: bool,
        cap_back: bool,
        mesh: &mut MeshData,
        ss: SecondaryStructure,
    ) {
        let n_ring = coords.len();
        let n_pts = centers.len();
        if n_pts < 2 {
            return;
        }

        let base = mesh.vertices.len() as u32;

        // 预计算所有 binormal (T × N)
        let mut binormals = vec![Vec3::ZERO; n_pts];
        for i in 0..n_pts {
            binormals[i] = tangents[i].cross(normals[i]).normalize_or_zero();
        }

        // === SIMD 批量生成顶点（8 点并行）===
        let mut i = 0;
        while i + 8 <= n_pts {
            let frame = Frame8::load(centers, tangents, normals, i);
            let (cx, cy, cz, tx, ty, tz, nx, ny, nz) = frame.to_simd();

            // binormal = T × N (SIMD 叉积)
            let bx = ty * nz - tz * ny;
            let by = tz * nx - tx * nz;
            let bz = tx * ny - ty * nx;
            let b_sq = bx * bx + by * by + bz * bz;
            let b_len = b_sq.sqrt();
            let b_len_safe = b_len + f32x8::splat(1e-8); // 防除零
            let bx = bx / b_len_safe;
            let by = by / b_len_safe;
            let bz = bz / b_len_safe;

            // local_t: 手动构造数组（修复 Range 错误）
            let denom = (n_pts.saturating_sub(1)) as f32;
            let local_t_arr = if denom > 0.0 {
                [
                    (i as f32) / denom,
                    (i as f32 + 1.0) / denom,
                    (i as f32 + 2.0) / denom,
                    (i as f32 + 3.0) / denom,
                    (i as f32 + 4.0) / denom,
                    (i as f32 + 5.0) / denom,
                    (i as f32 + 6.0) / denom,
                    (i as f32 + 7.0) / denom,
                ]
            } else {
                [0.0; 8]
            };
            let local_t = f32x8::new(local_t_arr);

            // has_arrow: 模拟 horizontal_max (用 to_array + 标量 max)
            let has_arrow = if let Some(arrow_back) = arrow_back {
                let local_t_arr = local_t.to_array(); // 修复：用 to_array() 提取
                let max_t = local_t_arr.iter().copied().fold(0.0f32, f32::max); // 标量 max 模拟
                max_t >= 0.5
            } else {
                false
            };

            for ring_idx in 0..n_ring {
                let mut off_n = f32x8::splat(coords[ring_idx][0]); // 标量 → SIMD splat
                let mut off_b = f32x8::splat(coords[ring_idx][1]);

                if has_arrow {
                    let back = arrow_back.unwrap()[ring_idx];
                    let back_n = f32x8::splat(back[0]);
                    let back_b = f32x8::splat(back[1]);

                    // arrow_progress: 完整 SIMD 计算（修复类型不匹配）
                    let arrow_progress = ((local_t - f32x8::splat(0.5)) * f32x8::splat(2.0))
                        .max(f32x8::splat(0.0)) // 用 splat 确保类型一致
                        .min(f32x8::splat(1.0));
                    let t = arrow_progress;
                    let one_t = f32x8::ONE - t; // ONE 是内置常量

                    // 修复：全部 f32x8 操作（off_n * one_t 等）
                    off_n = off_n * one_t + back_n * t;
                    off_b = off_b * one_t + back_b * t;
                }

                // 顶点位置（SIMD）
                let px = cx + nx * off_n + bx * off_b;
                let py = cy + ny * off_n + by * off_b;
                let pz = cz + nz * off_n + bz * off_b;

                // 法线（根据 ss 类型，SIMD 化）
                let (nx_out, ny_out, nz_out) = match ss {
                    SecondaryStructure::Helix => {
                        ellipse_normal_simd(
                            (nx, ny, nz),
                            (bx, by, bz),
                            (off_n, off_b),
                            1.0,  // width
                            0.25, // height
                        )
                    }
                    SecondaryStructure::Sheet => match ring_idx {
                        0 | 1 => (bx, by, bz),
                        2 | 3 => (-nx, -ny, -nz),
                        4 | 5 => (-bx, -by, -bz),
                        6 | 7 => (nx, ny, nz),
                        _ => (nx, ny, nz),
                    },
                    _ => {
                        // 通用 normalize
                        let nn_x = nx * off_n + bx * off_b;
                        let nn_y = ny * off_n + by * off_b;
                        let nn_z = nz * off_n + bz * off_b;
                        let nn_sq = nn_x * nn_x + nn_y * nn_y + nn_z * nn_z;
                        let nn_len = nn_sq.sqrt() + f32x8::splat(1e-8);
                        (nn_x / nn_len, nn_y / nn_len, nn_z / nn_len)
                    }
                };

                // 写入：用 to_array() 提取（修复索引错误）
                let px_arr = px.to_array(); // [f32; 8]
                let py_arr = py.to_array();
                let pz_arr = pz.to_array();
                let nx_arr = nx_out.to_array();
                let ny_arr = ny_out.to_array();
                let nz_arr = nz_out.to_array();

                for j in 0..8 {
                    if i + j >= n_pts {
                        break;
                    }
                    let pos = Vec3::new(px_arr[j], py_arr[j], pz_arr[j]); // 修复：数组索引
                    let nor = Vec3::new(nx_arr[j], ny_arr[j], nz_arr[j]).normalize_or_zero();
                    mesh.vertices.push(pos);
                    mesh.normals.push(nor);
                }
            }
            i += 8;
        }

        // 尾巴用标量处理（极少）
        for ii in i..n_pts {
            let c = centers[ii];
            let n = normals[ii];
            let b = binormals[ii];
            let local_t = ii as f32 / (n_pts - 1) as f32;
            let arrow_progress = if arrow_back.is_some() {
                ((local_t - 0.5) * 2.0).max(0.0).min(1.0)
            } else {
                0.0
            };

            for ring_idx in 0..n_ring {
                let mut off = coords[ring_idx];
                if arrow_progress > 0.0 && arrow_back.is_some() {
                    let back = arrow_back.unwrap()[ring_idx];
                    let t = arrow_progress;
                    off[0] = off[0] * (1.0 - t) + back[0] * t;
                    off[1] = off[1] * (1.0 - t) + back[1] * t;
                }
                let pos = c + n * off[0] + b * off[1];
                let nor = match ss {
                    SecondaryStructure::Helix => ellipse_normal(n, b, off, 1.0, 0.25),
                    SecondaryStructure::Sheet => match ring_idx {
                        0 | 1 => b,
                        2 | 3 => -n,
                        4 | 5 => -b,
                        6 | 7 => n,
                        _ => n,
                    }
                    .normalize(),
                    _ => (n * off[0] + b * off[1]).normalize_or_zero(),
                };
                mesh.vertices.push(pos);
                mesh.normals.push(nor);
            }
        }

        // === 索引生成（保持原逻辑，完全无瓶颈）===
        let verts_per_ring = n_ring as u32;
        for i in 0..n_pts - 1 {
            for r in 0..n_ring {
                let r_next = (r + 1) % n_ring;
                let i0 = i as u32 * verts_per_ring + r as u32;
                let i1 = i as u32 * verts_per_ring + r_next as u32;
                let j0 = (i + 1) as u32 * verts_per_ring + r as u32;
                let j1 = (i + 1) as u32 * verts_per_ring + r_next as u32;

                let a = base + i0;
                let b = base + i1;
                let c = base + j1;
                let d = base + j0;

                mesh.indices.extend_from_slice(&[a, b, d, b, c, d]);
            }
        }

        // 端盖（保持不变，几乎不耗时）
        let mut cap = |start_pt: usize, tangent: Vec3, outward: bool| {
            let center = base + (start_pt * n_ring) as u32;
            let normal = if outward { tangent } else { -tangent };
            for r in 1..n_ring - 1 {
                let a = center;
                let b = center + r as u32;
                let c = center + (r + 1) as u32;
                let v_ab = mesh.vertices[b as usize] - mesh.vertices[a as usize];
                let v_ac = mesh.vertices[c as usize] - mesh.vertices[a as usize];
                if normal.dot(v_ab.cross(v_ac)) > 0.0 {
                    mesh.indices.extend_from_slice(&[a, c, b]);
                } else {
                    mesh.indices.extend_from_slice(&[a, b, c]);
                }
            }
        };

        // 删除你原来的 cap lambda，替换为：
        if cap_front {
            // 链头：用第一个点的 normal 作为 hint，保证和前一段连续
            let hint = normals[0];
            add_cap(mesh, base, 0, n_ring, tangents[0], false, hint);
        }
        if cap_back {
            // 链尾：用最后一个点的 normal 作为 hint
            let hint = normals[n_pts - 1];
            add_cap(
                mesh,
                base,
                n_pts - 1,
                n_ring,
                tangents[n_pts - 1],
                true,
                hint,
            );
        }
    }
}

fn ellipse_normal(n: Vec3, b: Vec3, off: [f32; 2], width: f32, height: f32) -> Vec3 {
    let x = off[0];
    let y = off[1];
    let nx = x / (width * width);
    let ny = y / (height * height);
    let nor = n * nx + b * ny;
    nor.normalize_or_zero()
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

// Helix
static HELIX_SECTION: Lazy<RibbonXSection> =
    Lazy::new(|| RibbonXSection::smooth_circle().scale(1.0, 0.25).as_helix());

// Sheet
static SHEET_SECTION: Lazy<RibbonXSection> = Lazy::new(|| {
    // Arrow: 前半段宽 1.0 → 0.0，后半段保持 0.0
    RibbonXSection::smooth_circle()
        .scale(1.0, 0.25)
        .arrow(1.0, 1.0, 0.0, 1.2)
});

// Coil
static COIL_SECTION: Lazy<RibbonXSection> =
    Lazy::new(|| RibbonXSection::smooth_circle().scale(0.2, 0.2));

struct RibbonXSection {
    coords: Vec<[f32; 2]>,               // 基础 2D 轮廓
    arrow_coords: Option<Vec<[f32; 2]>>, // 为 Sheet 箭头准备的第二套轮廓
    ss: SecondaryStructure,
    _smooth: bool,
}

impl RibbonXSection {
    fn smooth_circle() -> Self {
        let mut coords = Vec::new();
        let n = 32;
        for i in 0..n {
            let a = (i as f32) / (n as f32) * std::f32::consts::TAU;
            coords.push([a.cos(), a.sin()]);
        }
        Self {
            coords: coords.to_vec(),
            arrow_coords: None,
            ss: SecondaryStructure::Coil,
            _smooth: true,
        }
    }

    fn scale(mut self, sx: f32, sy: f32) -> Self {
        for c in &mut self.coords {
            c[0] *= sx;
            c[1] *= sy;
        }
        if let Some(ac) = self.arrow_coords.as_mut() {
            for c in ac {
                c[0] *= sx;
                c[1] *= sy;
            }
        }
        self
    }

    fn arrow(mut self, sx1: f32, sy1: f32, sx2: f32, sy2: f32) -> Self {
        let mut back = self.coords.clone();
        for c in &mut self.coords {
            c[0] *= sx1;
            c[1] *= sy1;
        }
        for c in &mut back {
            c[0] *= sx2;
            c[1] *= sy2;
        }
        self.arrow_coords = Some(back);
        Self {
            coords: [
                [0.2, 1.0],
                [-0.2, 1.0],
                [-0.2, 1.0],
                [-0.2, -1.0],
                [-0.2, -1.0],
                [0.2, -1.0],
                [0.2, -1.0],
                [0.2, 1.0],
            ]
            .into(),
            arrow_coords: None,
            ss: SecondaryStructure::Sheet,
            _smooth: true,
        }
    }

    fn as_helix(mut self) -> Self {
        self.ss = SecondaryStructure::Helix;
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SimdVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Frame8 {
    cx: [f32; 8],
    cy: [f32; 8],
    cz: [f32; 8],
    tx: [f32; 8],
    ty: [f32; 8],
    tz: [f32; 8],
    nx: [f32; 8],
    ny: [f32; 8],
    nz: [f32; 8],
}

impl Frame8 {
    #[inline(always)]
    fn load(centers: &[Vec3], tangents: &[Vec3], normals: &[Vec3], i: usize) -> Self {
        let mut f = Frame8::zeroed(); // bytemuck::Zeroable
        let end = (i + 8).min(centers.len());
        for (j, idx) in (i..end).enumerate() {
            f.cx[j] = centers[idx].x;
            f.cy[j] = centers[idx].y;
            f.cz[j] = centers[idx].z;
            f.tx[j] = tangents[idx].x;
            f.ty[j] = tangents[idx].y;
            f.tz[j] = tangents[idx].z;
            f.nx[j] = normals[idx].x;
            f.ny[j] = normals[idx].y;
            f.nz[j] = normals[idx].z;
        }
        f
    }

    #[inline(always)]
    fn to_simd(
        &self,
    ) -> (
        f32x8,
        f32x8,
        f32x8,
        f32x8,
        f32x8,
        f32x8,
        f32x8,
        f32x8,
        f32x8,
    ) {
        (
            f32x8::new(self.cx), // 正确：new([f32; 8])
            f32x8::new(self.cy),
            f32x8::new(self.cz),
            f32x8::new(self.tx),
            f32x8::new(self.ty),
            f32x8::new(self.tz),
            f32x8::new(self.nx),
            f32x8::new(self.ny),
            f32x8::new(self.nz),
        )
    }
}

#[inline(always)]
fn ellipse_normal_simd(
    n: (f32x8, f32x8, f32x8), // normal: (nx, ny, nz)
    b: (f32x8, f32x8, f32x8), // binormal
    off: (f32x8, f32x8),      // off[0] = N方向偏移, off[1] = B方向偏移
    width: f32,
    height: f32,
) -> (f32x8, f32x8, f32x8) {
    let (nx, ny, nz) = n;
    let (bx, by, bz) = b;
    let (off_n, off_b) = off;

    // 椭圆法线公式：(x/w², y/h²) → 归一化
    let nx_ell = off_n / f32x8::splat(width * width);
    let ny_ell = off_b / f32x8::splat(height * height);

    // 合成法线：N * nx_ell + B * ny_ell
    let nx_out = nx * nx_ell + bx * ny_ell;
    let ny_out = ny * nx_ell + by * ny_ell;
    let nz_out = nz * nx_ell + bz * ny_ell;

    // 归一化
    let len_sq = nx_out * nx_out + ny_out * ny_out + nz_out * nz_out;
    let len = len_sq.sqrt() + f32x8::splat(1e-10);
    (nx_out / len, ny_out / len, nz_out / len)
}

/// 完美闭合端盖：星形三角化 + 法线连续 + 防反面
fn add_cap(
    mesh: &mut MeshData,
    base: u32,
    start_pt: usize,
    n_ring: usize,
    tangent: Vec3,
    outward: bool,     // true = 向外（链尾），false = 向内（链头）
    normal_hint: Vec3, // 来自前一个/后一个点的法线，保证连续
) {
    let center_idx = base + (start_pt * n_ring) as u32;
    let normal = if outward { tangent } else { -tangent };

    // 用 normal_hint 修正初始法线方向（关键！避免法线翻转）
    let mut fixed_normal = normal;
    if normal.dot(normal_hint) < 0.0 {
        fixed_normal = -normal;
    }

    // 星形三角化（带正确朝向判断）
    for r in 0..n_ring {
        let a = center_idx;
        let b = center_idx + r as u32;
        let c = center_idx + (r as u32 + 1) % n_ring as u32;

        let v_ab = mesh.vertices[b as usize] - mesh.vertices[a as usize];
        let v_ac = mesh.vertices[c as usize] - mesh.vertices[a as usize];
        let face_normal = v_ab.cross(v_ac);

        if fixed_normal.dot(face_normal) > 0.0 {
            mesh.indices.extend_from_slice(&[a, b, c]);
        } else {
            mesh.indices.extend_from_slice(&[c, a, b]);
        }
    }
}
