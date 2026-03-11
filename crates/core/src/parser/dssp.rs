// DSSP secondary structure assignment
//
// Ported and adapted from ChimeraX:
// https://github.com/RBVI/ChimeraX/blob/develop/src/bundles/atomic_lib/atomic_cpp/atomstruct_cpp/CompSS.cpp
//
// Original ChimeraX code (c) 2004-2025 Regents of the University of California.
// Licensed under a BSD-style license; see the original repository for details.
//
// Original DSSP algorithm:
// Kabsch & Sander, Biopolymers 22:2577-2637 (1983)
// https://doi.org/10.1002/bip.360221211

use crate::parser::utils::{Residue, SecondaryStructure};
use glam::Vec3;
use kiddo::{KdTree, SquaredEuclidean};
use na_seq::AminoAcid;
use rayon::prelude::*;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::{AtomicBool, Ordering};

type Point3 = [f64; 3];

// 氢键矩阵：O(1) 访问，内存友好
type HBondMatrix = Vec<Vec<AtomicBool>>;

// 残基坐标缓存（带氢）
#[derive(Clone)]
struct ResidueCoords {
    n: Point3,
    c: Point3,
    o: Point3,
    // ca: Point3,
    h: Option<Point3>,
    is_proline: bool,
}

// 氢键类型
#[derive(Debug, Clone, Copy)]
pub struct HydrogenBond {
    pub donor_idx: usize,
    pub acceptor_idx: usize,
    pub energy: f32,
}

// 二级结构计算器
pub struct SecondaryStructureCalculator {
    pub hbond_cutoff: f32,        // 氢键能量阈值
    pub min_helix_length: usize,  // 最小螺旋长度
    pub min_strand_length: usize, // 最小β链长度
}

impl Default for SecondaryStructureCalculator {
    fn default() -> Self {
        Self {
            hbond_cutoff: -0.5,   // 默认氢键能量阈值
            min_helix_length: 3,  // 默认最小螺旋长度
            min_strand_length: 2, // 默认最小β链长度
        }
    }
}

impl SecondaryStructureCalculator {
    const DSSP_3DONOR: u32 = 0x0001;
    const DSSP_3ACCEPTOR: u32 = 0x0002;
    const DSSP_3HELIX: u32 = 0x0008;

    const DSSP_4DONOR: u32 = 0x0010;
    const DSSP_4ACCEPTOR: u32 = 0x0020;
    const DSSP_4HELIX: u32 = 0x0080;

    const DSSP_5DONOR: u32 = 0x0100;
    const DSSP_5ACCEPTOR: u32 = 0x0200;
    const DSSP_5HELIX: u32 = 0x0800;

    pub fn new() -> Self {
        Self::default()
    }

    /// 计算一条链的二级结构
    pub fn compute_secondary_structure(&self, residues: &[Residue]) -> Vec<SecondaryStructure> {
        if residues.len() < 2 {
            return vec![SecondaryStructure::Coil; residues.len()];
        }

        // 关键：先补氢
        let residues_with_h = self.add_imide_hydrogens(residues);

        // 关键：用加速版查找氢键
        let hbonds = self.find_hydrogen_bonds(&residues_with_h);

        // 下面这些函数全部改为接受 &HBondMatrix
        let mut flags = vec![0u32; residues.len()];
        let n_res = residues.len();

        self.find_turns(3, &mut flags, &hbonds, n_res);
        self.mark_helices(3, &mut flags, n_res);
        self.find_turns(4, &mut flags, &hbonds, n_res);
        self.mark_helices(4, &mut flags, n_res);
        self.find_turns(5, &mut flags, &hbonds, n_res);
        self.mark_helices(5, &mut flags, n_res);

        let mut helices = self.collect_helix_regions(&flags, n_res);
        SecondaryStructureCalculator::extend_helix_ends(&mut helices, &flags, n_res);
        let strands = self.find_strands(&hbonds, n_res); // 也用 matrix 版

        self.assign_ss_labels(n_res, &helices, &strands, &mut flags)
    }

    /// 补全缺失的亚氨基氢原子
    fn add_imide_hydrogens(&self, residues: &[Residue]) -> Vec<Residue> {
        let mut result = residues.to_vec();

        for i in 1..result.len() {
            if result[i].h.is_some() {
                continue;
            }

            if let Some(prev_res) = result.get(i - 1) {
                if let Some(h_coord) = self.calculate_imide_hydrogen(&result[i], prev_res) {
                    result[i].h = Some(h_coord);
                }
            }
        }

        result
    }
    /// 计算亚氨基氢原子位置
    fn calculate_imide_hydrogen(&self, current: &Residue, previous: &Residue) -> Option<Vec3> {
        let n_coord = current.n;
        let ca_coord = current.ca;
        let c_coord = previous.c;
        let o_coord = previous.o;

        // 计算向量
        let n_to_ca = ca_coord - &n_coord;
        let n_to_c = c_coord - &n_coord;
        let c_to_o = o_coord - &c_coord;

        let _ = n_to_ca.normalize();
        let _ = n_to_c.normalize();
        let _ = c_to_o.normalize();

        // 计算角平分线
        let cac_bisect = Vec3 {
            x: n_to_ca.x + n_to_c.x,
            y: n_to_ca.y + n_to_c.y,
            z: n_to_ca.z + n_to_c.z,
        };
        let _ = cac_bisect.normalize();

        // 计算氢原子方向
        let h_direction = Vec3 {
            x: cac_bisect.x + c_to_o.x,
            y: cac_bisect.y + c_to_o.y,
            z: cac_bisect.z + c_to_o.z,
        };
        let _ = h_direction.normalize();

        // 氢键长度约1.01Å
        let nh_length = 1.01;
        Some(Vec3 {
            x: n_coord.x - h_direction.x * nh_length,
            y: n_coord.y - h_direction.y * nh_length,
            z: n_coord.z - h_direction.z * nh_length,
        })
    }

    fn find_hydrogen_bonds(&self, residues: &[Residue]) -> HBondMatrix {
        let n = residues.len();
        let matrix: Vec<Vec<AtomicBool>> = (0..n)
            .map(|_| (0..n).map(|_| AtomicBool::new(false)).collect())
            .collect();
        let mut coords = Vec::with_capacity(n);

        // Step 1: 构建坐标缓存 + KDTree（只放 N 原子）
        let mut points = Vec::with_capacity(n);
        for (i, res) in residues.iter().enumerate() {
            let h = res.h.map(|v| [v.x as f64, v.y as f64, v.z as f64]);
            let coord = ResidueCoords {
                n: [res.n.x as f64, res.n.y as f64, res.n.z as f64],
                c: [res.c.x as f64, res.c.y as f64, res.c.z as f64],
                o: [res.o.x as f64, res.o.y as f64, res.o.z as f64],
                // ca: [res.ca.x as f64, res.ca.y as f64, res.ca.z as f64],
                h,
                is_proline: res.residue_type == AminoAcid::Pro,
            };
            coords.push(coord);
            points.push((i, coords[i].n));
        }

        // 构建 KDTree
        let mut tree: KdTree<f64, 3> = KdTree::new();
        for &(idx, pt) in points.iter() {
            tree.add(&pt, idx as u64);
        }

        // Step 2: 只搜索 10Å 内的 N 原子（关键加速！）
        (0..n).into_par_iter().for_each(|i| {
            let query_point = coords[i].n;
            let neighbors = tree.within::<SquaredEuclidean>(&query_point, 400.0); // 20² = 400

            for neighbor in neighbors {
                let j = neighbor.item as usize;
                if j <= i + 1 {
                    continue;
                } // 避免重复 + 跳过相邻

                // 正向：i 的 C=O ... H-N j
                if !coords[j].is_proline && coords[j].h.is_some() {
                    let h = coords[j].h.unwrap();
                    let r_cn_sq = dist_sq(coords[i].c, coords[j].n); // 加这行
                    if r_cn_sq > 49.0 {
                        continue;
                    }
                    let energy = self.calc_hbond_energy(coords[i].c, coords[i].o, coords[j].n, h);
                    if energy < self.hbond_cutoff as f64 {
                        matrix[j][i].store(true, Ordering::Relaxed);
                    }
                }

                // 反向：j 的 C=O ... H-N i
                if !coords[i].is_proline && coords[i].h.is_some() {
                    let h = coords[i].h.unwrap();
                    let r_cn_sq = dist_sq(coords[i].c, coords[j].n); // 加这行
                    if r_cn_sq > 49.0 {
                        continue;
                    }
                    let energy = self.calc_hbond_energy(coords[j].c, coords[j].o, coords[i].n, h);
                    if energy < self.hbond_cutoff as f64 {
                        matrix[i][j].store(true, Ordering::Relaxed);
                    }
                }
            }
        });

        matrix
    }

    #[inline]
    fn calc_hbond_energy(&self, c: Point3, o: Point3, n: Point3, h: Point3) -> f64 {
        let r_on = dist_sq(o, n).sqrt();
        let r_ch = dist_sq(c, h).sqrt();
        let r_oh = dist_sq(o, h).sqrt();
        let r_cn = dist_sq(c, n).sqrt();

        let q1 = 0.42_f64;
        let q2 = 0.20_f64;
        let f = 332.0_f64;

        q1 * q2 * (1.0 / r_on + 1.0 / r_ch - 1.0 / r_oh - 1.0 / r_cn) * f
    }

    fn find_turns(&self, n: usize, flags: &mut [u32], hbonds: &HBondMatrix, n_res: usize) {
        let donor = match n {
            3 => Self::DSSP_3DONOR,
            4 => Self::DSSP_4DONOR,
            5 => Self::DSSP_5DONOR,
            _ => return,
        };
        let acceptor = match n {
            3 => Self::DSSP_3ACCEPTOR,
            4 => Self::DSSP_4ACCEPTOR,
            5 => Self::DSSP_5ACCEPTOR,
            _ => return,
        };

        for i in 0..n_res.saturating_sub(n) {
            let j = i + n;
            if hbonds[i][j].load(Ordering::Relaxed) || hbonds[j][i].load(Ordering::Relaxed) {
                flags[i] |= acceptor;
                flags[j] |= donor;
                // 删: for k in 1..n { flags[i + k] |= ... }  // GAP 多余
            }
        }
    }

    // ──────────────────────────────────────────────────────────────
    // 1. mark_helices_fast —— 完全对应原版 C++ 的 mark_helices()
    // ──────────────────────────────────────────────────────────────
    fn mark_helices(&self, n: usize, flags: &mut [u32], n_res: usize) {
        let acceptor = match n {
            3 => Self::DSSP_3ACCEPTOR,
            4 => Self::DSSP_4ACCEPTOR,
            5 => Self::DSSP_5ACCEPTOR,
            _ => return,
        };
        let helix_flag = match n {
            3 => Self::DSSP_3HELIX,
            4 => Self::DSSP_4HELIX,
            5 => Self::DSSP_5HELIX,
            _ => return,
        };

        // 原版逻辑：只要连续两个 acceptor，就把中间 n 个残基标记为该类型螺旋
        // 例如 α-螺旋 (n=4): i-1 是 acceptor 且 i 是 acceptor → i 到 i+3 都打上 4HELIX
        for i in 1..n_res.saturating_sub(n - 1) {
            if (flags[i - 1] & acceptor) != 0 && (flags[i] & acceptor) != 0 {
                for j in 0..n {
                    if i + j < n_res {
                        flags[i + j] |= helix_flag;
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────
    // 完全对齐 ChimeraX 的 find_bridges() + ladder + bulge 合并
    // ──────────────────────────────────────────────────────────────
    fn find_strands(&self, hbonds: &HBondMatrix, n_res: usize) -> Vec<(usize, usize)> {
        // 0 = none, 1 = 'P' (parallel), 2 = 'A' (antiparallel)
        let bridge_type: Vec<Vec<AtomicU8>> = (0..n_res)
            .map(|_| (0..n_res).map(|_| AtomicU8::new(0)).collect())
            .collect();

        // Step 1: 标记所有单桥（一次遍历）
        (0..n_res).into_par_iter().for_each(|i| {
            for j in (i + 2)..n_res {
                // ── 平行桥（两种方向都要判断！）──
                let mut is_parallel = false;

                // 方向 1: i-1 ← j   且   j ← i+1   (NH j → CO (i-1)) && (NH (i+1) → CO j)
                if i > 0
                    && i + 1 < n_res
                    && hbonds[j][i - 1].load(Ordering::Relaxed)
                    && hbonds[i + 1][j].load(Ordering::Relaxed)
                {
                    is_parallel = true;
                }

                // 方向 2: j-1 ← i   且   i ← j+1   (NH i → CO (j-1)) && (NH (j+1) → CO i)
                if j > 0
                    && j + 1 < n_res
                    && hbonds[i][j - 1].load(Ordering::Relaxed)
                    && hbonds[j + 1][i].load(Ordering::Relaxed)
                {
                    is_parallel = true;
                }

                // ── 反平行桥（两种模式都要判断！）──
                let mut is_antiparallel = false;
                // 经典双向氢键
                if hbonds[i][j].load(Ordering::Relaxed) && hbonds[j][i].load(Ordering::Relaxed) {
                    is_antiparallel = true;
                }
                // 错一位的反平行桥（非常常见！）
                if i > 0
                    && j > 0
                    && i + 1 < n_res
                    && j + 1 < n_res
                    && hbonds[i + 1][j - 1].load(Ordering::Relaxed)
                    && hbonds[j + 1][i - 1].load(Ordering::Relaxed)
                {
                    is_antiparallel = true;
                }

                if is_parallel {
                    bridge_type[i][j].store(1, Ordering::Relaxed);
                }
                if is_antiparallel {
                    bridge_type[i][j].store(2, Ordering::Relaxed);
                }
            }
        });

        // Step 2: 扫描出所有连续 ladder（和原版一模一样）
        let mut ladders = Vec::new();
        for i in 0..n_res {
            for j in (i + 2)..n_res {
                match bridge_type[i][j].load(Ordering::Relaxed) {
                    1 => {
                        // parallel ladder
                        let mut k = 0;
                        while i + k < n_res
                            && j + k < n_res
                            && bridge_type[i + k][j + k].load(Ordering::Relaxed) == 1
                        {
                            bridge_type[i + k][j + k].store(0, Ordering::Relaxed); // mark as used
                            k += 1;
                        }
                        if k >= self.min_strand_length {
                            ladders.push((i, i + k - 1, j, j + k - 1, true)); // true = parallel
                        }
                    }
                    2 => {
                        // antiparallel ladder
                        let mut k = 0;
                        while i + k < n_res
                            && j >= k
                            && bridge_type[i + k][j - k].load(Ordering::Relaxed) == 2
                        {
                            bridge_type[i + k][j - k].store(0, Ordering::Relaxed);
                            k += 1;
                        }
                        if k >= self.min_strand_length {
                            ladders.push((i, i + k - 1, j - k + 1, j, false));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Step 3: Beta-bulge 合并（while find_beta_bulge）
        let mut changed = true;
        while changed {
            changed = false;
            let mut idx = 0;
            while idx + 1 < ladders.len() {
                let l1 = ladders[idx];
                let mut jdx = idx + 1;
                while jdx < ladders.len() {
                    if let Some(merged) = self.merge_bulge_ladders(l1, ladders[jdx]) {
                        ladders[idx] = merged;
                        ladders.remove(jdx);
                        changed = true;
                    } else {
                        jdx += 1;
                    }
                }
                idx += 1;
            }
        }

        // Step 4: 过滤短 ladder（两边都要够长）
        ladders.retain(|&(s1, e1, s2, e2, _)| {
            (e1 - s1 + 1) >= self.min_strand_length && (e2 - s2 + 1) >= self.min_strand_length
        });

        // ── 原版 ChimeraX 关键一步：N→C 排序，确保 strand 编号顺序正确 ──
        let mut res_ranges: Vec<(usize, usize)> = Vec::new();
        for &(s1, e1, s2, e2, _) in &ladders {
            res_ranges.push((s1, e1));
            res_ranges.push((s2, e2));
        }
        res_ranges.sort(); // ← 这一行是灵魂！

        // 现在用有序的 res_ranges 标记 strand_mask（你原来的逻辑完全正确）
        let mut strand_mask = vec![false; n_res];
        for &(start, end) in &res_ranges {
            for x in start..=end {
                if x < n_res {
                    strand_mask[x] = true;
                }
            }
        }

        // // Step 5: 合并所有参与 β-strand 的残基区间
        // let mut strand_mask = vec![false; n_res];
        // for &(s1, e1, s2, e2, _) in &ladders {
        //     for x in s1..=e1 {
        //         if x < n_res {
        //             strand_mask[x] = true;
        //         }
        //     }
        //     for x in s2..=e2 {
        //         if x < n_res {
        //             strand_mask[x] = true;
        //         }
        //     }
        // }

        // 连续 true → strand
        let mut result = Vec::new();
        let mut start = None;
        for i in 0..n_res {
            if strand_mask[i] {
                if start.is_none() {
                    start = Some(i);
                }
            } else if let Some(s) = start {
                if i - s >= self.min_strand_length {
                    result.push((s, i - 1));
                }
                start = None;
            }
        }
        if let Some(s) = start {
            if n_res - s >= self.min_strand_length {
                result.push((s, n_res - 1));
            }
        }
        result
    }

    // ──────────────────────────────────────────────────────────────
    // Beta-bulge 合并（100% 对应原版 merge_bulge）
    // ladder = (start1, end1, start2, end2, is_parallel)
    // ──────────────────────────────────────────────────────────────
    fn merge_bulge_ladders(
        &self,
        l1: (usize, usize, usize, usize, bool),
        l2: (usize, usize, usize, usize, bool),
    ) -> Option<(usize, usize, usize, usize, bool)> {
        let (mut s1, mut e1, mut s2, mut e2, para1) = l1;
        let (mut c1, mut c2, mut d1, mut d2, para2) = l2;

        if para1 != para2 {
            return None;
        }

        // 保证 l1 在前
        if s1 > c1 {
            std::mem::swap(&mut s1, &mut c1);
            std::mem::swap(&mut e1, &mut c2);
            std::mem::swap(&mut s2, &mut d1);
            std::mem::swap(&mut e2, &mut d2);
        }

        let d0 = c1 as isize - e1 as isize - 1; // 链1 上的间隙
        let gap2 = if para1 {
            d1 as isize - e2 as isize - 1 // 平行：l2.start2 - l1.end2
        } else {
            s2 as isize - d2 as isize - 1 // 反平行：l1.start2 - l2.end2
        };

        // K&S 定义：两边最多一个 1-res bulge，另一边最多 4-res
        if d0 < 0 || d0 > 4 || gap2 < 0 || gap2 > 4 || (d0 > 1 && gap2 > 1) {
            return None;
        }

        let new_s1 = s1;
        let new_e1 = c2;
        let new_s2 = s2.min(d1); // 修复: 用 d1 (l2 start2) 而非 c1
        let new_e2 = e2.max(d2); // 统一 max
        Some((new_s1, new_e1, new_s2, new_e2, para1))
    }

    /// 合并螺旋区域：优先级 4 > 5 > 3，允许单残基中断，处理边界
    fn collect_helix_regions(&self, flags: &[u32], n_res: usize) -> Vec<(usize, usize)> {
        let mut helices = vec![];
        let mut first = None;
        let mut cur_helix_type = 0;
        let mut acc_only_run = 0;
        let mut in_initial_acc_only = false;

        for i in 0..n_res {
            let f = flags[i];

            // 优先级：4 > 5 > 3，且 4 和 5 都算 α-螺旋
            let (helix_type, acc_only) = if (f & Self::DSSP_4HELIX) != 0 {
                (4, (f & Self::DSSP_4DONOR) == 0)
            } else if (f & Self::DSSP_5HELIX) != 0 {
                (4, (f & Self::DSSP_5DONOR) == 0) // 5 也算 4
            } else if (f & Self::DSSP_3HELIX) != 0 {
                (3, (f & Self::DSSP_3DONOR) == 0)
            } else {
                (0, false)
            };

            // 必须有实际的氢键标记才算
            let helix_flags = if helix_type == 4 {
                Self::DSSP_4ACCEPTOR | Self::DSSP_4DONOR | Self::DSSP_5ACCEPTOR | Self::DSSP_5DONOR
            } else if helix_type == 3 {
                Self::DSSP_3ACCEPTOR | Self::DSSP_3DONOR
            } else {
                0
            };

            if helix_type > 0 && (f & helix_flags) != 0 {
                // —— 真正的螺旋残基 ——
                if first.is_none() {
                    first = Some(i);
                    cur_helix_type = helix_type;
                    in_initial_acc_only = true;
                } else if helix_type != cur_helix_type {
                    if let Some(s) = first {
                        if i - s >= self.min_helix_length {
                            helices.push((s, i - 1));
                        }
                    }
                    first = Some(i);
                    cur_helix_type = helix_type;
                    acc_only_run = 0;
                    in_initial_acc_only = true;
                }

                // acceptor-only 处理（和原版一模一样）
                if in_initial_acc_only {
                    in_initial_acc_only = acc_only || (i == first.unwrap());
                } else if acc_only {
                    if acc_only_run > 0 {
                        // 连续两个 >> → 截断
                        if let Some(s) = first {
                            if i - 1 - s >= self.min_helix_length {
                                helices.push((s, i - 2));
                            }
                        }
                        first = Some(i - 1);
                        cur_helix_type = helix_type;
                        acc_only_run = 0;
                        in_initial_acc_only = true;
                    } else {
                        acc_only_run += 1;
                    }
                } else {
                    acc_only_run = 0;
                }
            } else if let Some(s) = first {
                // 螺旋结束
                if i - s >= self.min_helix_length {
                    helices.push((s, i - 1));
                }
                first = None;
                acc_only_run = 0;
                in_initial_acc_only = false;
            }
        }

        if let Some(s) = first {
            if n_res - s >= self.min_helix_length {
                helices.push((s, n_res - 1));
            }
        }

        helices
    }

    // 在 collect_helix_regions 返回后，加上 helix capping
    fn extend_helix_ends(helices: &mut Vec<(usize, usize)>, flags: &[u32], n_res: usize) {
        for (start, end) in helices.iter_mut() {
            // N-端延伸
            if *start > 0 && (flags[*start - 1] & (Self::DSSP_4DONOR | Self::DSSP_5DONOR)) != 0 {
                *start -= 1;
            }
            // C-端延伸
            if *end + 1 < n_res
                && (flags[*end + 1] & (Self::DSSP_4ACCEPTOR | Self::DSSP_5ACCEPTOR)) != 0
            {
                *end += 1;
            }
        }
    }

    /// 分配二级结构标签
    fn assign_ss_labels(
        &self,
        num_residues: usize,
        helices: &[(usize, usize)],
        strands: &[(usize, usize)],
        flags: &[u32],
    ) -> Vec<SecondaryStructure> {
        let mut labels = vec![SecondaryStructure::Coil; num_residues];

        // 优先分配螺旋(螺旋优先级高于β折叠)
        for &(start, end) in helices {
            for i in start..=end {
                if i < num_residues {
                    labels[i] = SecondaryStructure::Helix;
                }
            }
        }

        // 分配β折叠
        for &(start, end) in strands {
            for i in start..=end {
                if i < num_residues && labels[i] == SecondaryStructure::Coil {
                    labels[i] = SecondaryStructure::Sheet;
                }
            }
        }

        // 识别转角(基于氢键模式，这里简化处理)
        self.identify_turns(&mut labels, flags);

        labels
    }

    /// 识别转角
    fn identify_turns(&self, labels: &mut [SecondaryStructure], flags: &[u32]) {
        // 第一步：基于原始 turn 信息标记 Turn（原版 ChimeraX 就是这么干的！）
        // 包括：
        // - 'X'：同时是 donor 和 acceptor（双向氢键）
        // - '>'：只有 acceptor
        // - '<'：只有 donor
        // - '3','4','5'：中间 GAP 残基
        for i in 0..labels.len() {
            if labels[i] != SecondaryStructure::Coil {
                continue;
            }

            let f = flags[i];

            // 3-turn 相关
            if (f & (Self::DSSP_3DONOR | Self::DSSP_3ACCEPTOR)) != 0 {
                labels[i] = SecondaryStructure::Turn;
                continue;
            }

            // 4-turn 相关（最常见的 β-turn / γ-turn）
            if (f & (Self::DSSP_4DONOR | Self::DSSP_4ACCEPTOR)) != 0 {
                labels[i] = SecondaryStructure::Turn;
                continue;
            }

            // 5-turn（较少见）
            if (f & (Self::DSSP_5DONOR | Self::DSSP_5ACCEPTOR)) != 0 {
                labels[i] = SecondaryStructure::Turn;
            }
        }

        // 第二步：你的“短 Coil 片段”补丁保留！（原版没有，但很好用）
        // 它能抓住一些 DSSP 没标记的紧凑转角（尤其是 β-hairpin）
        let mut i = 0;
        while i < labels.len() {
            if labels[i] == SecondaryStructure::Coil {
                let start = i;
                while i < labels.len() && labels[i] == SecondaryStructure::Coil {
                    i += 1;
                }
                let length = i - start;
                if length >= 2 && length <= 4 {
                    for j in start..i {
                        labels[j] = SecondaryStructure::Turn;
                    }
                }
            } else {
                i += 1;
            }
        }
    }
}

#[inline]
fn dist_sq(a: Point3, b: Point3) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}
