use crate::Shape;
use crate::parser::mmcif::Chain;
use crate::parser::mmcif::MmCif;
use crate::parser::utils::{
    Residue, ResidueType::AminoAcid, RibbonResidueInfo, SecondaryStructure,
};
use crate::utils::{Material, MeshData, Stylable};
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3, Vec4};
use na_seq::AtomTypeInRes;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub chains: Vec<Chain>,
    pub center: Vec3,

    pub style: Material,
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
            style: Material {
                opacity: 1.0,
                visible: true,
                roughness: 0.7,
                ..Default::default()
            },
        })
    }
}

impl Stylable for Protein {
    fn style_mut(&mut self) -> &mut Material {
        &mut self.style
    }
}

impl Protein {
    pub fn init_secondary_structure(&mut self) {
        for chain in &mut self.chains {
            chain.init_ss();
        }
    }

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

    fn estimate_tangents(&self, positions: &[Vec3]) -> Vec<Vec3> {
        let n = positions.len();
        let mut tangents = Vec::with_capacity(n);
        for i in 0..n {
            let prev = if i > 0 {
                positions[i - 1]
            } else {
                positions[i]
            };
            let next = if i + 1 < n {
                positions[i + 1]
            } else {
                positions[i]
            };
            tangents.push((next - prev).normalize_or_zero());
        }
        tangents
    }

    fn default_face_normal(&self, residue: &Residue, tangent: Vec3) -> Vec3 {
        let mut face = project_perpendicular(residue.o - residue.c, tangent);
        if face.length_squared() <= 1e-6 {
            face = project_perpendicular(residue.o - residue.ca, tangent);
        }
        if face.length_squared() <= 1e-6 {
            face = fallback_face_normal(tangent);
        }
        face.normalize_or_zero()
    }

    fn apply_helix_face_normals(
        &self,
        face_normals: &mut [Vec3],
        ca_positions: &[Vec3],
        ribbon_info: &[RibbonResidueInfo],
        residue_tangents: &[Vec3],
    ) {
        // Keep the helix path on the CA spiral and only steer the wide face toward the helix axis.
        let mut helix_start = 0;
        while helix_start < ribbon_info.len() {
            let Some(helix_id) = ribbon_info[helix_start].helix_id else {
                helix_start += 1;
                continue;
            };

            let mut helix_end = helix_start + 1;
            while helix_end < ribbon_info.len() && ribbon_info[helix_end].helix_id == Some(helix_id)
            {
                helix_end += 1;
            }

            let axis_direction = helix_axis_direction(&ca_positions[helix_start..helix_end]);
            let axis_center = average_vec3(&ca_positions[helix_start..helix_end]);
            for residue_idx in helix_start..helix_end {
                let axis_point = axis_center
                    + axis_direction
                        * (ca_positions[residue_idx] - axis_center).dot(axis_direction);
                let inward = project_perpendicular(
                    axis_point - ca_positions[residue_idx],
                    residue_tangents[residue_idx],
                );
                if inward.length_squared() > 1e-6 {
                    face_normals[residue_idx] = inward.normalize();
                }
            }

            helix_start = helix_end;
        }
    }

    fn apply_helix_face_samples(
        &self,
        face_samples: &mut [Vec3],
        centers: &[Vec3],
        tangents: &[Vec3],
        sample_helix_ids: &[Option<usize>],
    ) {
        let mut helix_start = 0;
        while helix_start < sample_helix_ids.len() {
            let Some(helix_id) = sample_helix_ids[helix_start] else {
                helix_start += 1;
                continue;
            };

            let mut helix_end = helix_start + 1;
            while helix_end < sample_helix_ids.len()
                && sample_helix_ids[helix_end] == Some(helix_id)
            {
                helix_end += 1;
            }

            let axis_direction = helix_axis_direction(&centers[helix_start..helix_end]);
            let axis_center = average_vec3(&centers[helix_start..helix_end]);
            for sample_idx in helix_start..helix_end {
                let axis_point = axis_center
                    + axis_direction * (centers[sample_idx] - axis_center).dot(axis_direction);
                let inward =
                    project_perpendicular(axis_point - centers[sample_idx], tangents[sample_idx]);
                if inward.length_squared() > 1e-6 {
                    face_samples[sample_idx] = inward.normalize();
                }
            }

            for sample_idx in (helix_start + 1)..helix_end {
                if face_samples[sample_idx - 1].dot(face_samples[sample_idx]) < 0.0 {
                    face_samples[sample_idx] = -face_samples[sample_idx];
                }
            }

            helix_start = helix_end;
        }
    }

    fn apply_sheet_face_normals(
        &self,
        face_normals: &mut [Vec3],
        ca_positions: &[Vec3],
        ribbon_info: &[RibbonResidueInfo],
        residue_tangents: &[Vec3],
    ) {
        // Use one low-frequency plane normal per sheet group so each strand stays locally flat.
        let sheet_strands = collect_sheet_strands(ribbon_info);
        let mut sheet_ids: Vec<usize> = ribbon_info
            .iter()
            .filter_map(|info| info.sheet_id)
            .collect();
        sheet_ids.sort_unstable();
        sheet_ids.dedup();

        for sheet_id in sheet_ids {
            let strands: Vec<&SheetStrand> = sheet_strands
                .iter()
                .filter(|strand| strand.sheet_id == sheet_id)
                .collect();
            let Some(plane_normal) =
                self.estimate_sheet_plane_normal(&strands, ca_positions, residue_tangents)
            else {
                continue;
            };

            for strand in strands {
                for residue_idx in strand.start..=strand.end {
                    face_normals[residue_idx] = plane_normal;
                }
            }
        }
    }

    fn estimate_sheet_plane_normal(
        &self,
        strands: &[&SheetStrand],
        ca_positions: &[Vec3],
        residue_tangents: &[Vec3],
    ) -> Option<Vec3> {
        if strands.len() < 2 {
            return None;
        }

        let mut long_axis = Vec3::ZERO;
        let mut reference_tangent = None;
        for strand in strands {
            let tangent =
                average_vec3(&residue_tangents[strand.start..=strand.end]).normalize_or_zero();
            if tangent.length_squared() <= 1e-6 {
                continue;
            }

            let tangent = if let Some(reference) = reference_tangent {
                if tangent.dot(reference) < 0.0 {
                    -tangent
                } else {
                    tangent
                }
            } else {
                reference_tangent = Some(tangent);
                tangent
            };
            long_axis += tangent;
        }
        if long_axis.length_squared() <= 1e-6 {
            return None;
        }
        long_axis = long_axis.normalize();

        let centroids: Vec<Vec3> = strands
            .iter()
            .map(|strand| average_vec3(&ca_positions[strand.start..=strand.end]))
            .collect();

        let mut cross_axis = Vec3::ZERO;
        for (idx, centroid) in centroids.iter().enumerate() {
            let mut best = None;
            for (other_idx, other_centroid) in centroids.iter().enumerate() {
                if idx == other_idx {
                    continue;
                }

                let delta = project_perpendicular(*other_centroid - *centroid, long_axis);
                let dist2 = delta.length_squared();
                if dist2 <= 1e-6 {
                    continue;
                }

                if best.is_none_or(|(_, best_dist2)| dist2 < best_dist2) {
                    best = Some((delta, dist2));
                }
            }

            if let Some((delta, _)) = best {
                let mut delta = delta.normalize();
                if cross_axis.length_squared() > 1e-6 && cross_axis.dot(delta) < 0.0 {
                    delta = -delta;
                }
                cross_axis += delta;
            }
        }

        if cross_axis.length_squared() <= 1e-6 {
            return None;
        }

        let plane_normal = long_axis.cross(cross_axis).normalize_or_zero();
        (plane_normal.length_squared() > 1e-6).then_some(plane_normal)
    }

    fn stabilize_face_normals(&self, face_normals: &mut [Vec3], ribbon_info: &[RibbonResidueInfo]) {
        for i in 1..face_normals.len() {
            if face_normals[i].length_squared() <= 1e-6 {
                face_normals[i] = face_normals[i - 1];
                continue;
            }

            let prev = face_normals[i - 1];
            if prev.length_squared() <= 1e-6 {
                continue;
            }

            let same_band = ribbon_info[i].helix_id == ribbon_info[i - 1].helix_id
                || ribbon_info[i].sheet_id == ribbon_info[i - 1].sheet_id;
            if same_band && prev.dot(face_normals[i]) < 0.0 {
                face_normals[i] = -face_normals[i];
            }
        }
    }

    fn build_sampled_normals(
        &self,
        tangents: &[Vec3],
        face_samples: &[Vec3],
        sample_ss: &[SecondaryStructure],
    ) -> Vec<Vec3> {
        // The ribbon section's broad face is controlled by b = t x n, so we fit n from face targets.
        let mut normals = Vec::with_capacity(tangents.len());
        let initial_face = project_perpendicular(face_samples[0], tangents[0]).normalize_or_zero();
        let initial_normal = if initial_face.length_squared() > 1e-6 {
            initial_face.cross(tangents[0]).normalize_or_zero()
        } else {
            fallback_normal(tangents[0])
        };
        let mut current_normal = if initial_normal.length_squared() > 1e-6 {
            initial_normal
        } else {
            fallback_normal(tangents[0])
        };
        normals.push(current_normal);

        for i in 1..tangents.len() {
            let prev_t = tangents[i - 1];
            let curr_t = tangents[i];
            let axis = prev_t.cross(curr_t);
            if axis.length_squared() > 1e-6 {
                let angle = prev_t.angle_between(curr_t);
                let q = Quat::from_axis_angle(axis.normalize(), angle);
                current_normal = q * current_normal;
            }

            let desired_face = project_perpendicular(face_samples[i], curr_t);
            if desired_face.length_squared() > 1e-6 {
                let mut desired_normal = desired_face.normalize().cross(curr_t).normalize_or_zero();
                if current_normal.dot(desired_normal) < 0.0 {
                    desired_normal = -desired_normal;
                }

                let blend = match sample_ss[i] {
                    SecondaryStructure::Sheet => 0.92,
                    SecondaryStructure::Helix => 0.95,
                    SecondaryStructure::Coil | SecondaryStructure::Turn => 0.35,
                };
                current_normal =
                    (current_normal * (1.0 - blend) + desired_normal * blend).normalize_or_zero();
            }

            if current_normal.length_squared() <= 1e-6 {
                current_normal = fallback_normal(curr_t);
            }
            normals.push(current_normal);
        }

        normals
    }

    pub fn to_mesh(&self, scale: f32) -> MeshData {
        let mut final_mesh = MeshData::default();
        let pts_per_res = 5;

        self.chains.par_iter().for_each(|chain| {
            chain.get_ribbon_info();
        });

        let meshes: Vec<MeshData> = self
            .chains
            .par_iter()
            .filter_map(|chain| {
                let mut mesh = MeshData::default();
                let ribbon_info_all = chain.get_ribbon_info();
                let filtered: Vec<(usize, &Residue)> = chain
                    .residues
                    .iter()
                    .enumerate()
                    .filter(|(_, residue)| residue.ca.length_squared() > 1e-6)
                    .collect();

                if filtered.len() < 2 {
                    return None;
                }

                let residues: Vec<&Residue> =
                    filtered.iter().map(|(_, residue)| *residue).collect();
                let ribbon_info: Vec<RibbonResidueInfo> = filtered
                    .iter()
                    .map(|(idx, _)| ribbon_info_all[*idx])
                    .collect();
                let ss: Vec<SecondaryStructure> = ribbon_info.iter().map(|info| info.ss).collect();

                let centers = residues
                    .iter()
                    .map(|residue| residue.ca)
                    .collect::<Vec<_>>();
                let residue_tangents = self.estimate_tangents(&centers);
                let mut face_normals: Vec<Vec3> = residues
                    .iter()
                    .enumerate()
                    .map(|(idx, residue)| self.default_face_normal(residue, residue_tangents[idx]))
                    .collect();

                self.apply_helix_face_normals(
                    &mut face_normals,
                    &centers,
                    &ribbon_info,
                    &residue_tangents,
                );
                self.apply_sheet_face_normals(
                    &mut face_normals,
                    &centers,
                    &ribbon_info,
                    &residue_tangents,
                );
                self.stabilize_face_normals(&mut face_normals, &ribbon_info);

                let centers = self.catmull_rom_chain(&centers, pts_per_res);
                let mut face_samples = self.catmull_rom_chain(&face_normals, pts_per_res);
                let n = centers.len();
                let mut tangents = Vec::with_capacity(n);
                for i in 0..n {
                    let p0 = if i > 0 { centers[i - 1] } else { centers[0] };
                    let p1 = centers[i];
                    let p2 = if i + 1 < n {
                        centers[i + 1]
                    } else {
                        centers[i]
                    };
                    let p3 = if i + 2 < n { centers[i + 2] } else { p2 };
                    tangents.push(catmull_rom_tangent(p0, p1, p2, p3).normalize_or_zero());
                }

                let sample_ss: Vec<SecondaryStructure> = (0..n)
                    .map(|sample_idx| {
                        ss[((sample_idx + pts_per_res / 2) / pts_per_res).min(ss.len() - 1)]
                    })
                    .collect();
                let sample_helix_ids: Vec<Option<usize>> = (0..n)
                    .map(|sample_idx| {
                        ribbon_info[((sample_idx + pts_per_res / 2) / pts_per_res)
                            .min(ribbon_info.len() - 1)]
                        .helix_id
                    })
                    .collect();
                self.apply_helix_face_samples(
                    &mut face_samples,
                    &centers,
                    &tangents,
                    &sample_helix_ids,
                );
                let normals = self.build_sampled_normals(&tangents, &face_samples, &sample_ss);

                for (res_idx, ss_type) in ss.iter().enumerate() {
                    if res_idx == ss.len() - 1 {
                        continue;
                    }

                    let i0 = res_idx * pts_per_res;
                    let i1 = ((res_idx + 1) * pts_per_res + 1).min(n);
                    let mid = (i0 + i1) / 2;

                    let section_front = match ss_type {
                        SecondaryStructure::Helix => &*HELIX_SECTION,
                        SecondaryStructure::Sheet => &*SHEET_SECTION,
                        _ => &*COIL_SECTION,
                    };
                    let section_back = section_front;

                    let front_centers = &centers[i0..mid + 1];
                    let front_tangents = &tangents[i0..mid + 1];
                    let front_normals = &normals[i0..mid + 1];
                    if front_centers.len() > 1 {
                        let scales = vec![1.0; front_centers.len()];
                        mesh.append(&section_front.extrude(
                            front_centers,
                            front_tangents,
                            front_normals,
                            &scales,
                        ));
                    }

                    let back_centers = &centers[mid..i1];
                    let back_tangents = &tangents[mid..i1];
                    let back_normals = &normals[mid..i1];
                    if back_centers.len() > 1 {
                        let scales = vec![1.0; back_centers.len()];
                        mesh.append(&section_back.extrude(
                            back_centers,
                            back_tangents,
                            back_normals,
                            &scales,
                        ));
                    }

                    if res_idx == 0 {
                        mesh.append(&section_front.add_flat_cap(
                            &centers[i0],
                            &tangents[i0],
                            &normals[i0],
                            1.0,
                        ));
                    } else if !matches!(
                        (&ss[res_idx - 1], ss_type),
                        (SecondaryStructure::Helix, SecondaryStructure::Helix)
                            | (SecondaryStructure::Sheet, SecondaryStructure::Sheet)
                            | (
                                SecondaryStructure::Coil | SecondaryStructure::Turn,
                                SecondaryStructure::Coil | SecondaryStructure::Turn
                            )
                    ) {
                        let (section, is_first) = cap_use(&ss[res_idx - 1], ss_type);
                        let cap_tangent = if is_first {
                            -tangents[i0]
                        } else {
                            tangents[i0]
                        };
                        mesh.append(&section.add_flat_cap(
                            &centers[i0],
                            &cap_tangent,
                            &normals[i0],
                            1.0,
                        ));
                    }

                    if res_idx == ss.len() - 2 {
                        mesh.append(&section_back.add_flat_cap(
                            &centers[i1 - 1],
                            &-tangents[i1 - 1],
                            &normals[i1 - 1],
                            1.0,
                        ));
                    }
                }

                for vertex in &mut mesh.vertices {
                    *vertex *= scale;
                }

                let color = match self.style.color {
                    Some(color) => Vec4::new(color[0], color[1], color[2], self.style.opacity),
                    None => Vec4::new(1.0, 1.0, 1.0, self.style.opacity),
                };
                mesh.colors = Some(vec![color; mesh.vertices.len()]);
                mesh.material_params = Some(vec![
                    [self.style.roughness, self.style.metallic,];
                    mesh.vertices.len()
                ]);

                Some(mesh)
            })
            .collect();

        for mesh in meshes {
            final_mesh.append(&mesh);
        }

        final_mesh
    }
}

// 加这个函数
#[derive(Clone, Copy)]
struct SheetStrand {
    sheet_id: usize,
    start: usize,
    end: usize,
}

fn collect_sheet_strands(ribbon_info: &[RibbonResidueInfo]) -> Vec<SheetStrand> {
    let mut strands = Vec::new();
    let mut start = 0;

    while start < ribbon_info.len() {
        let info = ribbon_info[start];
        if info.ss != SecondaryStructure::Sheet {
            start += 1;
            continue;
        }

        let Some(sheet_id) = info.sheet_id else {
            start += 1;
            continue;
        };

        let mut end = start + 1;
        while end < ribbon_info.len()
            && ribbon_info[end].ss == SecondaryStructure::Sheet
            && ribbon_info[end].sheet_id == Some(sheet_id)
        {
            end += 1;
        }

        strands.push(SheetStrand {
            sheet_id,
            start,
            end: end - 1,
        });
        start = end;
    }

    strands
}

fn fallback_normal(tangent: Vec3) -> Vec3 {
    if tangent.dot(Vec3::Z).abs() < 0.98 {
        tangent.cross(Vec3::Z).normalize_or_zero()
    } else {
        tangent.cross(Vec3::X).normalize_or_zero()
    }
}

fn fallback_face_normal(tangent: Vec3) -> Vec3 {
    let side = fallback_normal(tangent);
    tangent.cross(side).normalize_or_zero()
}

fn project_perpendicular(vector: Vec3, tangent: Vec3) -> Vec3 {
    vector - tangent * vector.dot(tangent)
}

fn average_vec3(values: &[Vec3]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for value in values {
        sum += *value;
    }
    sum / values.len().max(1) as f32
}

fn helix_axis_direction(points: &[Vec3]) -> Vec3 {
    if points.len() < 2 {
        return Vec3::Y;
    }

    let mut axis = Vec3::ZERO;
    if points.len() >= 5 {
        for i in 0..=(points.len() - 5) {
            axis += points[i + 4] - points[i];
        }
    }
    if axis.length_squared() <= 1e-6 {
        axis = points[points.len() - 1] - points[0];
    }
    if axis.length_squared() <= 1e-6 {
        axis = Vec3::Y;
    }
    axis.normalize_or_zero()
}

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
            material_params: None,
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
            material_params: None,
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
    make_round_section(1.3, 0.25, 24)
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
