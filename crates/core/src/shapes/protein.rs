use crate::Shape;
use crate::parser::mmcif::Chain;
use crate::parser::mmcif::MmCif;
use crate::parser::utils::{
    Residue, ResidueType, ResidueType::AminoAcid, RibbonResidueInfo, SecondaryStructure,
};
use crate::shapes::Stick;
use crate::utils::{Material, MeshData, Stylable};
use bytemuck::{Pod, Zeroable};
use cosmolkit::{
    BioStructure, ChainSourceIds, ResidueKind as CosmolkitResidueKind,
    read_mmcif_atom_site_subset_from_str,
};
use glam::{Quat, Vec3, Vec4};
use na_seq::AtomTypeInRes;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub chains: Vec<Chain>,
    pub center: Vec3,

    pub style: Material,
    #[serde(default)]
    pub color_mode: ProteinColorMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProteinColorMode {
    #[default]
    Uniform,
    RainbowResidues,
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseMmCifError {
    #[error("Failed to parse MmCif data: '{0}'")]
    ParsingError(String),
}

impl Protein {
    pub fn from_mmcif(mmcif: &str) -> Result<Self, ParseMmCifError> {
        match read_mmcif_atom_site_subset_from_str(mmcif) {
            Ok(structure) => Self::from_biostructure(&structure),
            Err(cosmolkit_error) => {
                let protein_data = MmCif::new(mmcif).map_err(|fallback_error| {
                    ParseMmCifError::ParsingError(format!(
                        "COSMolKit mmCIF parser failed: {cosmolkit_error}; fallback parser failed: {fallback_error}"
                    ))
                })?;
                Self::new(protein_data)
            }
        }
    }

    pub fn from_pdb(pdb: &str) -> Result<Self, ParseMmCifError> {
        let structure = BioStructure::from_pdb_str(pdb)
            .map_err(|e| ParseMmCifError::ParsingError(e.to_string()))?;
        Self::from_biostructure(&structure)
    }

    fn from_biostructure(structure: &BioStructure) -> Result<Self, ParseMmCifError> {
        let mut chains = Vec::new();
        let mut centers = Vec::new();
        let positions = structure.coordinates().positions();

        for (chain_index, chain_row) in structure.chains().iter().enumerate() {
            let mut residues = Vec::new();
            let residue_start = chain_row.residue_span.start as usize;
            let residue_end = chain_row.residue_span.end() as usize;

            for residue_index in residue_start..residue_end {
                let residue_row = &structure.residues()[residue_index];
                if residue_row.kind != CosmolkitResidueKind::AminoAcid {
                    continue;
                }

                let amino_acid = match ResidueType::from_str(residue_row.name.as_str()) {
                    ResidueType::AminoAcid(aa) => aa,
                    _ => continue,
                };

                let atom_start = residue_row.atom_span.start as usize;
                let atom_end = residue_row.atom_span.end() as usize;
                let mut ca_opt = None;
                let mut c_opt = None;
                let mut n_opt = None;
                let mut o_opt = None;
                for atom_index in atom_start..atom_end {
                    let atom = &structure.atoms()[atom_index];
                    let Some(position) = positions.get(atom_index) else {
                        continue;
                    };
                    let position = Vec3::new(position[0], position[1], position[2]);
                    match biostructure_atom_name(&atom.name) {
                        "C" => c_opt = Some(position),
                        "N" => n_opt = Some(position),
                        "CA" => ca_opt = Some(position),
                        "O" => o_opt = Some(position),
                        _ => {}
                    }
                }

                let (Some(ca), Some(c), Some(n), Some(o)) = (ca_opt, c_opt, n_opt, o_opt) else {
                    continue;
                };

                centers.push(ca);
                residues.push(Residue {
                    residue_type: amino_acid,
                    ca,
                    c,
                    n,
                    o,
                    h: None,
                    sns: residue_row
                        .source
                        .seq_id
                        .map(|seq_id| seq_id.seq_num.max(0) as usize)
                        .unwrap_or(residue_index + 1),
                    ss: None,
                });
            }

            if !residues.is_empty() {
                chains.push(Chain::new(
                    biostructure_chain_id(chain_index, &chain_row.source),
                    residues,
                ));
            }
        }

        let center = average_center(&centers);

        Ok(Protein {
            chains,
            center,
            style: Material {
                opacity: 1.0,
                visible: true,
                roughness: 0.7,
                ..Default::default()
            },
            color_mode: ProteinColorMode::Uniform,
        })
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
        center = average_center(&centers);

        Ok(Protein {
            chains: chains,
            center: center,
            style: Material {
                opacity: 1.0,
                visible: true,
                roughness: 0.7,
                ..Default::default()
            },
            color_mode: ProteinColorMode::Uniform,
        })
    }
}

fn average_center(centers: &[Vec3]) -> Vec3 {
    if centers.is_empty() {
        return Vec3::ZERO;
    }
    centers.iter().copied().sum::<Vec3>() / centers.len() as f32
}

fn biostructure_atom_name(name: &cosmolkit::AtomName) -> &str {
    std::str::from_utf8(&name.0).unwrap_or("").trim()
}

fn biostructure_chain_id(chain_index: usize, source: &ChainSourceIds) -> String {
    source
        .auth_chain_id
        .or(source.label_asym_id)
        .map(|chain_id| chain_id.as_str().to_string())
        .unwrap_or_else(|| chain_index.to_string())
}

impl Stylable for Protein {
    fn style_mut(&mut self) -> &mut Material {
        &mut self.style
    }
}

crate::impl_stylable_methods!(Protein, style);

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

    pub fn rainbow_residues(mut self) -> Self {
        self.color_mode = ProteinColorMode::RainbowResidues;
        self.style.color = None;
        self
    }

    pub fn to_mesh(&self, scale: f32) -> MeshData {
        let mut final_mesh = MeshData::default();

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
                let residue_colors = self.residue_colors(chain, &filtered);
                mesh.append(&build_chimerax_ribbon_mesh(
                    &residues,
                    &ribbon_info,
                    residue_colors.as_deref(),
                ));

                for vertex in &mut mesh.vertices {
                    *vertex *= scale;
                }

                if mesh.colors.is_none() {
                    let color = match self.style.color {
                        Some(color) => Vec4::new(color[0], color[1], color[2], self.style.opacity),
                        None => Vec4::new(1.0, 1.0, 1.0, self.style.opacity),
                    };
                    mesh.colors = Some(vec![color; mesh.vertices.len()]);
                }
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

    fn residue_colors(&self, _chain: &Chain, filtered: &[(usize, &Residue)]) -> Option<Vec<Vec4>> {
        if self.style.color.is_some() || self.color_mode != ProteinColorMode::RainbowResidues {
            return None;
        }

        // BEGIN CHIMERAX PYTHON BODY: src/bundles/std_commands/src/rainbow.py :: rainbow
        // ChimeraX✔️✔️: rainbow delegates to color_sequential with level='residues'.
        // END CHIMERAX PYTHON BODY: src/bundles/std_commands/src/rainbow.py :: rainbow
        //
        // BEGIN CHIMERAX PYTHON BODY: src/bundles/std_commands/src/color.py :: _set_sequential_residue
        // ChimeraX✔️✔️: each chain is colored separately with the default rainbow colormap.
        // ChimeraX✔️✔️: residues in a chain are sampled with numpy.linspace(0.0, 1.0, len(residues)).
        // ChimeraX✔️✔️: the sampled rgba8 values become each residue's ribbon_color.
        // END CHIMERAX PYTHON BODY: src/bundles/std_commands/src/color.py :: _set_sequential_residue
        Some(chimerax_rainbow_residue_colors(
            filtered.len(),
            self.style.opacity,
        ))
    }
}

const CHIMERAX_RIBBON_DIVISIONS: usize = 20;
const BACKBONE_GAP_CA_DISTANCE: f32 = 4.7;
const BACKBONE_GAP_PEPTIDE_DISTANCE: f32 = 2.2;
const BACKBONE_GAP_DASH_RADIUS: f32 = 0.06;
const BACKBONE_GAP_DASH_LENGTH: f32 = 0.45;
const BACKBONE_GAP_DASH_SPACING: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RibbonClass {
    Coil,
    SheetStart,
    SheetMiddle,
    SheetEnd,
    HelixStart,
    HelixMiddle,
    HelixEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RibbonRef {
    Coil,
    Sheet,
    SheetArrow,
    Helix,
    HelixArrow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RibbonRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct RibbonXSection {
    kind: RibbonRef,
}

struct ChimeraXSectionManager;

impl ChimeraXSectionManager {
    fn assign(
        &self,
        rc0: RibbonClass,
        rc1: RibbonClass,
        rc2: RibbonClass,
    ) -> (RibbonRef, RibbonRef) {
        match rc1 {
            RibbonClass::SheetMiddle => (RibbonRef::Sheet, RibbonRef::Sheet),
            RibbonClass::HelixMiddle => (RibbonRef::Helix, RibbonRef::Helix),
            RibbonClass::Coil => (RibbonRef::Coil, RibbonRef::Coil),
            RibbonClass::SheetStart => match (rc0, rc2) {
                (
                    RibbonClass::Coil | RibbonClass::HelixEnd | RibbonClass::SheetEnd,
                    RibbonClass::SheetMiddle | RibbonClass::SheetEnd,
                ) => (RibbonRef::Coil, RibbonRef::Sheet),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
            RibbonClass::SheetEnd => match (rc0, rc2) {
                (
                    RibbonClass::SheetStart | RibbonClass::SheetMiddle,
                    RibbonClass::Coil | RibbonClass::HelixStart | RibbonClass::SheetStart,
                ) => (RibbonRef::SheetArrow, RibbonRef::Coil),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
            RibbonClass::HelixStart => match (rc0, rc2) {
                (
                    RibbonClass::Coil | RibbonClass::HelixEnd | RibbonClass::SheetEnd,
                    RibbonClass::HelixMiddle | RibbonClass::HelixEnd,
                ) => (RibbonRef::Coil, RibbonRef::Helix),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
            RibbonClass::HelixEnd => match (rc0, rc2) {
                (
                    RibbonClass::HelixStart | RibbonClass::HelixMiddle,
                    RibbonClass::Coil | RibbonClass::HelixStart | RibbonClass::SheetStart,
                ) => (RibbonRef::HelixArrow, RibbonRef::Coil),
                _ => (RibbonRef::Coil, RibbonRef::Coil),
            },
        }
    }

    fn xsection(&self, kind: RibbonRef) -> RibbonXSection {
        RibbonXSection { kind }
    }
}

fn build_chimerax_ribbon_mesh(
    residues: &[&Residue],
    ribbon_info: &[RibbonResidueInfo],
    residue_colors: Option<&[Vec4]>,
) -> MeshData {
    if residues.len() < 2 {
        return MeshData::default();
    }

    let mut mesh = MeshData::default();
    for range in contiguous_backbone_ranges(residues) {
        if range.end - range.start < 2 {
            continue;
        }

        let segment_colors = residue_colors.map(|colors| &colors[range.start..range.end]);
        mesh.append(&build_chimerax_ribbon_segment_mesh(
            &residues[range.start..range.end],
            &ribbon_info[range.start..range.end],
            segment_colors,
        ));
    }
    mesh.append(&build_backbone_gap_dashes(residues, residue_colors));
    mesh
}

fn build_chimerax_ribbon_segment_mesh(
    residues: &[&Residue],
    ribbon_info: &[RibbonResidueInfo],
    residue_colors: Option<&[Vec4]>,
) -> MeshData {
    let mut centers: Vec<Vec3> = residues.iter().map(|residue| residue.ca).collect();
    let mut guides: Vec<Vec3> = residues
        .iter()
        .map(|residue| residue.o - residue.c)
        .collect();
    let (res_class, _helix_ranges, sheet_ranges, display_ranges) = ribbon_ranges(ribbon_info);
    smooth_strands(&mut centers, &mut guides, &sheet_ranges);

    let coeffs = natural_cubic_spline_coefficients(&centers);
    let control_tangents = spline_control_tangents(&coeffs);
    let control_normals = path_plane_normals(&centers, &control_tangents);
    let smooth_twist = ribbon_smooth_twist(&res_class);
    let path = spline_path(
        &coeffs,
        &control_normals,
        &ribbon_flip_normals(ribbon_info),
        &smooth_twist,
        CHIMERAX_RIBBON_DIVISIONS,
    );

    let manager = ChimeraXSectionManager;
    let (xs_front, xs_back) = ribbon_crosssections(&res_class, &manager);
    ribbon_extrusions(
        &path.centers,
        &path.tangents,
        &path.normals,
        &display_ranges,
        residues.len(),
        &xs_front,
        &xs_back,
        residue_colors,
        &manager,
    )
}

fn contiguous_backbone_ranges(residues: &[&Residue]) -> Vec<RibbonRange> {
    if residues.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0;
    for i in 1..residues.len() {
        if !residue_backbone_connected(residues[i - 1], residues[i]) {
            ranges.push(RibbonRange { start, end: i });
            start = i;
        }
    }
    ranges.push(RibbonRange {
        start,
        end: residues.len(),
    });
    ranges
}

fn residue_backbone_connected(previous: &Residue, next: &Residue) -> bool {
    if previous.ca.distance(next.ca) > BACKBONE_GAP_CA_DISTANCE {
        return false;
    }

    let has_peptide_atoms = previous.c.length_squared() > 0.0 && next.n.length_squared() > 0.0;
    if has_peptide_atoms && previous.c.distance(next.n) > BACKBONE_GAP_PEPTIDE_DISTANCE {
        return false;
    }

    true
}

fn build_backbone_gap_dashes(residues: &[&Residue], residue_colors: Option<&[Vec4]>) -> MeshData {
    let mut mesh = MeshData::default();
    for i in 1..residues.len() {
        if residue_backbone_connected(residues[i - 1], residues[i]) {
            continue;
        }

        let color = residue_colors
            .and_then(|colors| colors.get(i - 1).zip(colors.get(i)))
            .map(|(previous, next)| (*previous + *next) * 0.5);
        mesh.append(&dashed_backbone_gap_mesh(
            residues[i - 1].ca,
            residues[i].ca,
            color,
        ));
    }
    mesh
}

fn dashed_backbone_gap_mesh(start: Vec3, end: Vec3, color: Option<Vec4>) -> MeshData {
    let delta = end - start;
    let length = delta.length();
    if length <= f32::EPSILON {
        return MeshData::default();
    }

    let direction = delta / length;
    let period = BACKBONE_GAP_DASH_LENGTH + BACKBONE_GAP_DASH_SPACING;
    let mut offset = 0.0;
    let mut mesh = MeshData::default();

    while offset < length {
        let dash_start = start + direction * offset;
        let dash_end = start + direction * (offset + BACKBONE_GAP_DASH_LENGTH).min(length);
        if dash_start.distance_squared(dash_end) > f32::EPSILON {
            let mut dash = Stick::new(
                dash_start.to_array(),
                dash_end.to_array(),
                BACKBONE_GAP_DASH_RADIUS,
            )
            .to_mesh(1.0);
            if let Some(color) = color {
                dash.colors = Some(vec![color; dash.vertices.len()]);
            } else {
                dash.colors = None;
            }
            dash.material_params = None;
            mesh.append(&dash);
        }
        offset += period;
    }

    mesh
}

fn chimerax_rainbow_residue_colors(residue_count: usize, opacity: f32) -> Vec<Vec4> {
    // BEGIN CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: _builtin_colormaps
    // ChimeraX✔️✔️: BuiltinColormaps["rainbow"] is blue, cyan, green, yellow, red.
    // ChimeraX✔️✔️: Colormap(None, colors) assigns evenly spaced values from 0.0 to 1.0.
    // END CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: _builtin_colormaps
    //
    // BEGIN CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: Colormap.interpolated_rgba8
    // ChimeraX✔️✔️: interpolated_rgba linearly interpolates in RGB/RGBA space, then rgba8 scales by 255.
    // END CHIMERAX PYTHON BODY: src/bundles/core/src/colors.py :: Colormap.interpolated_rgba8
    const STOPS: [Vec3; 5] = [
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    ];

    if residue_count == 0 {
        return Vec::new();
    }

    (0..residue_count)
        .map(|i| {
            let value = if residue_count == 1 {
                0.0
            } else {
                i as f32 / (residue_count - 1) as f32
            };
            let scaled = value * (STOPS.len() - 1) as f32;
            let lower = scaled.floor() as usize;
            let upper = lower.min(STOPS.len() - 1);
            let next = (lower + 1).min(STOPS.len() - 1);
            let t = scaled - lower as f32;
            let color = STOPS[upper] * (1.0 - t) + STOPS[next] * t;
            Vec4::new(
                chimerax_rgba8_channel_to_float(color.x),
                chimerax_rgba8_channel_to_float(color.y),
                chimerax_rgba8_channel_to_float(color.z),
                opacity,
            )
        })
        .collect()
}

fn chimerax_rgba8_channel_to_float(channel: f32) -> f32 {
    (channel.clamp(0.0, 1.0) * 255.0).trunc() / 255.0
}

fn ribbon_ranges(
    ribbon_info: &[RibbonResidueInfo],
) -> (
    Vec<RibbonClass>,
    Vec<RibbonRange>,
    Vec<RibbonRange>,
    Vec<RibbonRange>,
) {
    let mut classes = Vec::with_capacity(ribbon_info.len());
    let mut helix_ranges: Vec<RibbonRange> = Vec::new();
    let mut sheet_ranges: Vec<RibbonRange> = Vec::new();
    let mut was_sheet = false;
    let mut was_helix = false;
    let mut last_sheet_id = None;
    let mut last_helix_id = None;

    for (i, info) in ribbon_info.iter().enumerate() {
        let mut am_sheet = false;
        let mut am_helix = false;
        let class = if info.ss == SecondaryStructure::Sheet {
            am_sheet = true;
            if was_sheet && info.sheet_id == last_sheet_id {
                RibbonClass::SheetMiddle
            } else {
                if was_sheet {
                    end_strand(&mut classes, &mut sheet_ranges, i);
                }
                sheet_ranges.push(RibbonRange {
                    start: i,
                    end: usize::MAX,
                });
                RibbonClass::SheetStart
            }
        } else if info.ss == SecondaryStructure::Helix {
            am_helix = true;
            if was_helix && info.helix_id == last_helix_id {
                RibbonClass::HelixMiddle
            } else {
                if was_helix {
                    end_helix(&mut classes, &mut helix_ranges, i);
                }
                helix_ranges.push(RibbonRange {
                    start: i,
                    end: usize::MAX,
                });
                RibbonClass::HelixStart
            }
        } else {
            RibbonClass::Coil
        };

        if was_sheet && !am_sheet {
            end_strand(&mut classes, &mut sheet_ranges, i);
        } else if was_helix && !am_helix {
            end_helix(&mut classes, &mut helix_ranges, i);
        }

        classes.push(class);
        was_sheet = am_sheet;
        was_helix = am_helix;
        last_sheet_id = info.sheet_id;
        last_helix_id = info.helix_id;
    }

    if was_sheet {
        end_strand(&mut classes, &mut sheet_ranges, ribbon_info.len());
    } else if was_helix {
        end_helix(&mut classes, &mut helix_ranges, ribbon_info.len());
    }

    let display_ranges = if ribbon_info.is_empty() {
        Vec::new()
    } else {
        vec![RibbonRange {
            start: 0,
            end: ribbon_info.len(),
        }]
    };

    (classes, helix_ranges, sheet_ranges, display_ranges)
}

fn end_strand(classes: &mut [RibbonClass], ranges: &mut Vec<RibbonRange>, end: usize) {
    if classes.last() == Some(&RibbonClass::SheetStart) {
        if let Some(last) = classes.last_mut() {
            *last = RibbonClass::Coil;
        }
        ranges.pop();
    } else if let Some(last) = classes.last_mut() {
        *last = RibbonClass::SheetEnd;
        if let Some(range) = ranges.last_mut() {
            range.end = end;
        }
    }
}

fn end_helix(classes: &mut [RibbonClass], ranges: &mut Vec<RibbonRange>, end: usize) {
    if classes.last() == Some(&RibbonClass::HelixStart) {
        if let Some(last) = classes.last_mut() {
            *last = RibbonClass::Coil;
        }
        ranges.pop();
    } else if let Some(last) = classes.last_mut() {
        *last = RibbonClass::HelixEnd;
        if let Some(range) = ranges.last_mut() {
            range.end = end;
        }
    }
}

fn ribbon_crosssections(
    classes: &[RibbonClass],
    manager: &ChimeraXSectionManager,
) -> (Vec<RibbonXSection>, Vec<RibbonXSection>) {
    let mut front = Vec::with_capacity(classes.len());
    let mut back = Vec::with_capacity(classes.len());
    let mut rc0 = RibbonClass::Coil;

    for i in 0..classes.len() {
        let rc1 = classes[i];
        let rc2 = classes.get(i + 1).copied().unwrap_or(RibbonClass::Coil);
        let (f, b) = manager.assign(rc0, rc1, rc2);
        front.push(manager.xsection(f));
        back.push(manager.xsection(b));
        rc0 = rc1;
    }

    (front, back)
}

fn ribbon_smooth_twist(classes: &[RibbonClass]) -> Vec<bool> {
    let mut twist = Vec::with_capacity(classes.len());
    for i in 0..classes.len() {
        let rc0 = classes[i];
        let rc1 = classes.get(i + 1).copied().unwrap_or(RibbonClass::Coil);
        twist.push(smooth_twist_between(rc0, rc1));
    }
    if let Some(last) = twist.last_mut() {
        *last = false;
    }
    twist
}

fn smooth_twist_between(rc0: RibbonClass, rc1: RibbonClass) -> bool {
    rc0 == rc1
        || (is_sheet_class(rc0) && is_sheet_class(rc1))
        || (is_helix_class(rc0) && is_helix_class(rc1))
        || matches!(rc0, RibbonClass::HelixEnd | RibbonClass::SheetEnd)
        || matches!(rc1, RibbonClass::HelixStart | RibbonClass::SheetStart)
}

fn is_sheet_class(class: RibbonClass) -> bool {
    matches!(
        class,
        RibbonClass::SheetStart | RibbonClass::SheetMiddle | RibbonClass::SheetEnd
    )
}

fn is_helix_class(class: RibbonClass) -> bool {
    matches!(
        class,
        RibbonClass::HelixStart | RibbonClass::HelixMiddle | RibbonClass::HelixEnd
    )
}

fn ribbon_flip_normals(ribbon_info: &[RibbonResidueInfo]) -> Vec<bool> {
    let last = ribbon_info.len().saturating_sub(1);
    ribbon_info
        .iter()
        .enumerate()
        .map(|(i, info)| {
            !(info.ss == SecondaryStructure::Helix
                && (i == last || ribbon_info[i + 1].ss == SecondaryStructure::Helix))
        })
        .collect()
}

fn smooth_strands(centers: &mut [Vec3], guides: &mut [Vec3], sheet_ranges: &[RibbonRange]) {
    for range in sheet_ranges {
        smooth_strand(centers, guides, range.start, range.end);
    }
}

fn smooth_strand(centers: &mut [Vec3], guides: &mut [Vec3], start: usize, end: usize) {
    if end <= start || end - start <= 2 {
        return;
    }

    let original = centers[start..end].to_vec();
    let len = original.len();
    if len < 3 {
        return;
    }

    let mut ideal = vec![Vec3::ZERO; len];
    for i in 1..(len - 1) {
        ideal[i] = (original[i] * 2.0 + original[i - 1] + original[i + 1]) * 0.25;
    }
    if len == 3 {
        ideal[0] = original[0] - 0.99 * (ideal[1] - original[1]);
        ideal[len - 1] = original[len - 1] - 0.99 * (ideal[len - 2] - original[len - 2]);
    } else {
        ideal[0] = original[0] - (ideal[1] - original[1]);
        ideal[len - 1] = original[len - 1] - (ideal[len - 2] - original[len - 2]);
    }

    for i in 0..len {
        let delta_guide = guides[start + i];
        centers[start + i] = ideal[i];
        guides[start + i] = delta_guide;
    }
}

#[derive(Clone)]
struct RibbonPath {
    centers: Vec<Vec3>,
    tangents: Vec<Vec3>,
    normals: Vec<Vec3>,
}

fn natural_cubic_spline_coefficients(points: &[Vec3]) -> Vec<[Vec3; 4]> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }

    let mut extended = Vec::with_capacity(n + 2);
    extended.push(points[0] - (points[1] - points[0]));
    extended.extend_from_slice(points);
    extended.push(points[n - 1] + (points[n - 1] - points[n - 2]));

    let ne = extended.len();
    let mut deriv = vec![Vec3::ZERO; ne];
    for axis in 0..3 {
        let values: Vec<f32> = extended.iter().map(|p| p[axis]).collect();
        let mut a = vec![1.0f32; ne];
        let mut b = vec![4.0f32; ne];
        let c = vec![1.0f32; ne];
        let mut d = vec![0.0f32; ne];
        b[0] = 2.0;
        b[ne - 1] = 2.0;
        d[0] = values[1] - values[0];
        for i in 1..(ne - 1) {
            d[i] = 3.0 * (values[i + 1] - values[i - 1]);
        }
        d[ne - 1] = 3.0 * (values[ne - 1] - values[ne - 2]);
        let solved = tridiagonal(&mut a, &mut b, &c, &mut d);
        for i in 0..ne {
            deriv[i][axis] = solved[i];
        }
    }

    let mut coeffs = Vec::with_capacity(n - 1);
    for i in 1..(ne - 2) {
        let delta = extended[i + 1] - extended[i];
        coeffs.push([
            extended[i],
            deriv[i],
            delta * 3.0 - deriv[i] * 2.0 - deriv[i + 1],
            delta * -2.0 + deriv[i] + deriv[i + 1],
        ]);
    }
    coeffs
}

fn tridiagonal(a: &mut [f32], b: &mut [f32], c: &[f32], d: &mut [f32]) -> Vec<f32> {
    let n = a.len();
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m * c[i - 1];
        d[i] -= m * d[i - 1];
    }

    let mut x = vec![0.0; n];
    x[n - 1] = d[n - 1] / b[n - 1];
    for i in (0..(n - 1)).rev() {
        x[i] = (d[i] - c[i] * x[i + 1]) / b[i];
    }
    x
}

fn spline_control_tangents(coeffs: &[[Vec3; 4]]) -> Vec<Vec3> {
    if coeffs.is_empty() {
        return Vec::new();
    }
    let mut tangents: Vec<Vec3> = coeffs.iter().map(|c| c[1].normalize_or_zero()).collect();
    let last = coeffs.last().unwrap();
    tangents.push((last[1] + last[2] * 2.0 + last[3] * 3.0).normalize_or_zero());
    tangents
}

fn spline_path(
    coeffs: &[[Vec3; 4]],
    normals: &[Vec3],
    flip_normals: &[bool],
    twist: &[bool],
    divisions: usize,
) -> RibbonPath {
    let mut centers = Vec::new();
    let mut tangents = Vec::new();
    let mut path_normals = Vec::new();
    if coeffs.is_empty() {
        return RibbonPath {
            centers,
            tangents,
            normals: path_normals,
        };
    }

    let lead_n = divisions / 2;
    let (lead_c, lead_t) = cubic_path_points(&coeffs[0], -0.3, 0.0, lead_n + 1);
    let lead_nrm = parallel_transport_normals(&lead_t, normals[0], true);
    append_without_last(&mut centers, &lead_c);
    append_without_last(&mut tangents, &lead_t);
    append_without_last(&mut path_normals, &lead_nrm);

    let mut end_normal = normals[0];
    for seg in 0..coeffs.len() {
        let (seg_c, seg_t) = cubic_path_points(&coeffs[seg], 0.0, 1.0, divisions + 1);
        let start_normal = if seg == 0 { normals[0] } else { end_normal };
        let mut seg_n = parallel_transport_normals(&seg_t, start_normal, false);
        end_normal = normals[seg + 1];
        if twist.get(seg).copied().unwrap_or(false) {
            if flip_normals.get(seg).copied().unwrap_or(false)
                && need_normal_flip(*seg_n.last().unwrap(), end_normal, *seg_t.last().unwrap())
            {
                end_normal = -end_normal;
            }
            smooth_twist(&seg_t, &mut seg_n, end_normal);
        }
        append_without_last(&mut centers, &seg_c);
        append_without_last(&mut tangents, &seg_t);
        append_without_last(&mut path_normals, &seg_n);
    }

    let trail_n = (divisions + 1) / 2;
    let (trail_c, trail_t) = cubic_path_points(coeffs.last().unwrap(), 1.0, 1.3, trail_n);
    let trail_nrm = parallel_transport_normals(&trail_t, end_normal, false);
    centers.extend(trail_c);
    tangents.extend(trail_t);
    path_normals.extend(trail_nrm);

    RibbonPath {
        centers,
        tangents,
        normals: path_normals,
    }
}

fn append_without_last<T: Copy>(target: &mut Vec<T>, values: &[T]) {
    if !values.is_empty() {
        target.extend_from_slice(&values[..values.len() - 1]);
    }
}

fn cubic_path_points(coeff: &[Vec3; 4], tmin: f32, tmax: f32, n: usize) -> (Vec<Vec3>, Vec<Vec3>) {
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let mut coords = Vec::with_capacity(n);
    let mut tangents = Vec::with_capacity(n);
    let step = if n > 1 {
        (tmax - tmin) / (n - 1) as f32
    } else {
        0.0
    };
    for i in 0..n {
        let t = tmin + step * i as f32;
        coords.push(eval_cubic(coeff, t));
        tangents.push(eval_cubic_tangent(coeff, t).normalize_or_zero());
    }
    (coords, tangents)
}

fn eval_cubic(coeff: &[Vec3; 4], t: f32) -> Vec3 {
    coeff[0] + coeff[1] * t + coeff[2] * t * t + coeff[3] * t * t * t
}

fn eval_cubic_tangent(coeff: &[Vec3; 4], t: f32) -> Vec3 {
    coeff[1] + coeff[2] * (2.0 * t) + coeff[3] * (3.0 * t * t)
}

fn path_plane_normals(coords: &[Vec3], tangents: &[Vec3]) -> Vec<Vec3> {
    let n = coords.len();
    let mut normals = vec![Vec3::ZERO; n];
    if n < 2 {
        return normals;
    }

    for i in 1..(n - 1) {
        normals[i] = (coords[i - 1] - coords[i]).cross(coords[i + 1] - coords[i]);
    }
    normals[0] = normals[1];
    normals[n - 1] = normals[n - 2];

    let mut prev = None;
    for i in 0..n {
        let mut normal = project_perpendicular(normals[i], tangents[i]);
        let len = normal.length();
        if len > 1e-6 {
            if prev.is_some_and(|p: Vec3| normal.dot(p) < 0.0) {
                normal = -normal;
            }
            normal /= len;
            normals[i] = normal;
            prev = Some(normal);
        } else {
            normals[i] = Vec3::ZERO;
        }
    }
    replace_zero_normals(&mut normals, tangents);
    normals
}

fn replace_zero_normals(normals: &mut [Vec3], tangents: &[Vec3]) {
    let n = normals.len();
    let mut i = 0;
    while i < n {
        if normals[i].length_squared() > 1e-12 {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && normals[i].length_squared() <= 1e-12 {
            i += 1;
        }
        let end = i;
        if start == 0 && end < n {
            let fill = normals[end];
            for normal in normals.iter_mut().take(end) {
                *normal = fill;
            }
        } else if start > 0 && end < n {
            let n0 = normals[start - 1];
            let n1 = normals[end];
            for (offset, normal) in normals[start..end].iter_mut().enumerate() {
                let f = (offset + 1) as f32 / (end - start + 1) as f32;
                *normal = n0 * (1.0 - f) + n1 * f;
            }
        } else if start > 0 {
            let fill = normals[start - 1];
            for normal in normals.iter_mut().take(end).skip(start) {
                *normal = fill;
            }
        } else {
            for j in 0..n {
                normals[j] = fallback_normal(tangents[j]);
            }
        }
    }

    for i in 0..n {
        let normal = project_perpendicular(normals[i], tangents[i]).normalize_or_zero();
        normals[i] = if normal.length_squared() > 1e-12 {
            normal
        } else {
            fallback_normal(tangents[i])
        };
    }
}

fn parallel_transport_normals(tangents: &[Vec3], start_normal: Vec3, backwards: bool) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; tangents.len()];
    if tangents.is_empty() {
        return normals;
    }

    if backwards {
        let reversed_tangents: Vec<Vec3> = tangents.iter().rev().copied().collect();
        let mut reversed = parallel_transport_normals(&reversed_tangents, start_normal, false);
        reversed.reverse();
        return reversed;
    }

    let mut normal = project_perpendicular(start_normal, tangents[0]).normalize_or_zero();
    if normal.length_squared() <= 1e-12 {
        normal = fallback_normal(tangents[0]);
    }
    normals[0] = normal;

    for i in 1..tangents.len() {
        let prev_t = tangents[i - 1].normalize_or_zero();
        let curr_t = tangents[i].normalize_or_zero();
        let axis = prev_t.cross(curr_t);
        if axis.length_squared() > 1e-12 {
            let angle = prev_t.angle_between(curr_t);
            normal = Quat::from_axis_angle(axis.normalize(), angle) * normal;
        }
        normal = project_perpendicular(normal, curr_t).normalize_or_zero();
        if normal.length_squared() <= 1e-12 {
            normal = fallback_normal(curr_t);
        }
        normals[i] = normal;
    }

    normals
}

fn smooth_twist(tangents: &[Vec3], normals: &mut [Vec3], end_normal: Vec3) {
    if tangents.len() < 2 || normals.is_empty() {
        return;
    }
    let last = tangents.len() - 1;
    let angle = dihedral_angle(normals[last], end_normal, tangents[last]);
    for i in 0..normals.len() {
        let f = i as f32 / last as f32;
        let q = Quat::from_axis_angle(tangents[i].normalize_or_zero(), angle * f);
        normals[i] = (q * normals[i]).normalize_or_zero();
    }
}

fn need_normal_flip(transported_normal: Vec3, end_normal: Vec3, tangent: Vec3) -> bool {
    dihedral_angle(transported_normal, end_normal, tangent).abs() > 0.6 * std::f32::consts::PI
}

fn dihedral_angle(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    let axis = axis.normalize_or_zero();
    let from = project_perpendicular(from, axis).normalize_or_zero();
    let to = project_perpendicular(to, axis).normalize_or_zero();
    axis.dot(from.cross(to)).atan2(from.dot(to))
}

fn ribbon_extrusions(
    coords: &[Vec3],
    tangents: &[Vec3],
    normals: &[Vec3],
    ranges: &[RibbonRange],
    num_res: usize,
    xs_front: &[RibbonXSection],
    xs_back: &[RibbonXSection],
    residue_colors: Option<&[Vec4]>,
    manager: &ChimeraXSectionManager,
) -> MeshData {
    let mut mesh = MeshData::default();
    if num_res == 0 || coords.is_empty() {
        return mesh;
    }

    let nsp = coords.len() / num_res;
    let nlp = nsp / 2;
    let nrp = (nsp + 1) / 2;

    for range in ranges {
        let mut capped = true;
        for i in range.start..range.end {
            let residue_color = residue_colors.and_then(|colors| colors.get(i)).copied();
            let mid_cap = xs_front[i].kind != xs_back[i].kind;
            let s = i * nsp;
            let e = s + nlp + 1;
            let mut front_mesh = xs_front[i].extrude(
                &coords[s..e],
                &tangents[s..e],
                &normals[s..e],
                capped,
                mid_cap,
                manager,
            );
            if let Some(color) = residue_color {
                front_mesh.colors = Some(vec![color; front_mesh.vertices.len()]);
            }
            mesh.append(&front_mesh);

            let next_cap = if i + 1 == range.end {
                true
            } else {
                xs_back[i].kind != xs_front[i + 1].kind
            };
            let s = i * nsp + nlp;
            let e = if i < num_res - 1 {
                s + nrp + 1
            } else {
                s + nrp
            };
            let mut back_mesh = xs_back[i].extrude(
                &coords[s..e],
                &tangents[s..e],
                &normals[s..e],
                mid_cap,
                next_cap,
                manager,
            );
            if let Some(color) = residue_color {
                back_mesh.colors = Some(vec![color; back_mesh.vertices.len()]);
            }
            mesh.append(&back_mesh);
            capped = next_cap;
        }
    }

    mesh
}

impl RibbonXSection {
    fn extrude(
        self,
        centers: &[Vec3],
        tangents: &[Vec3],
        normals_3d: &[Vec3],
        cap_front: bool,
        cap_back: bool,
        manager: &ChimeraXSectionManager,
    ) -> MeshData {
        let section = section_coords(self.kind, manager);
        if section.faceted {
            extrude_faceted(&section, centers, tangents, normals_3d, cap_front, cap_back)
        } else {
            extrude_smooth(&section, centers, tangents, normals_3d, cap_front, cap_back)
        }
    }
}

struct SectionGeometry {
    coords: Vec<[f32; 2]>,
    coords2: Option<Vec<[f32; 2]>>,
    normals: Vec<[f32; 2]>,
    normals2: Option<Vec<[f32; 2]>>,
    faceted: bool,
    tessellation: Vec<[u32; 3]>,
}

fn section_coords(kind: RibbonRef, _manager: &ChimeraXSectionManager) -> SectionGeometry {
    match kind {
        RibbonRef::Coil => xsection_round((0.2, 0.2), 12, false),
        RibbonRef::Helix => xsection_round((1.0, 0.2), 12, false),
        RibbonRef::HelixArrow => xsection_round((1.0, 0.2), 12, false),
        RibbonRef::Sheet => xsection_square((1.0, 0.2)),
        RibbonRef::SheetArrow => xsection_square_arrow((2.0, 0.2), (0.2, 0.2)),
    }
}

fn xsection_round(scale: (f32, f32), sides: usize, faceted: bool) -> SectionGeometry {
    let mut coords = Vec::with_capacity(sides);
    let mut normals = Vec::with_capacity(sides);
    for i in 0..sides {
        let theta = (i as f32 / sides as f32) * std::f32::consts::TAU;
        let x = theta.cos();
        let y = theta.sin();
        coords.push([x * scale.0, y * scale.1]);
        normals.push([x * scale.1, y * scale.0]);
    }
    normalize_section_normals(&mut normals);
    SectionGeometry {
        coords,
        coords2: None,
        normals,
        normals2: None,
        faceted,
        tessellation: fan_tessellation(sides),
    }
}

fn xsection_square(scale: (f32, f32)) -> SectionGeometry {
    let coords = vec![
        [scale.0, scale.1],
        [-scale.0, scale.1],
        [-scale.0, -scale.1],
        [scale.0, -scale.1],
    ];
    let (normals, normals2) = generate_faceted_section_normals(&coords);
    SectionGeometry {
        coords,
        coords2: None,
        normals,
        normals2: Some(normals2),
        faceted: true,
        tessellation: vec![[0, 1, 2], [0, 2, 3]],
    }
}

fn xsection_square_arrow(start: (f32, f32), end: (f32, f32)) -> SectionGeometry {
    let mut section = xsection_square(start);
    section.coords2 = Some(vec![
        [end.0, end.1],
        [-end.0, end.1],
        [-end.0, -end.1],
        [end.0, -end.1],
    ]);
    section
}

fn generate_faceted_section_normals(coords: &[[f32; 2]]) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
    let num_coords = coords.len();
    let mut normals = vec![[0.0, 0.0]; num_coords];
    let mut normals2 = vec![[0.0, 0.0]; num_coords];
    for i in 0..num_coords {
        let j = (i + 1) % num_coords;
        let dx = coords[j][0] - coords[i][0];
        let dy = coords[j][1] - coords[i][1];
        normals[i] = [dy, -dx];
        normals2[j] = [dy, -dx];
    }
    normalize_section_normals(&mut normals);
    normalize_section_normals(&mut normals2);
    (normals, normals2)
}

fn normalize_section_normals(normals: &mut [[f32; 2]]) {
    for normal in normals {
        let len = (normal[0] * normal[0] + normal[1] * normal[1])
            .sqrt()
            .max(1e-6);
        normal[0] /= len;
        normal[1] /= len;
    }
}

fn fan_tessellation(n: usize) -> Vec<[u32; 3]> {
    (1..(n - 1))
        .map(|i| [0, i as u32, (i + 1) as u32])
        .collect()
}

fn extrude_smooth(
    section: &SectionGeometry,
    centers: &[Vec3],
    tangents: &[Vec3],
    normals_3d: &[Vec3],
    cap_front: bool,
    cap_back: bool,
) -> MeshData {
    let mut mesh = MeshData::default();
    let ns = section.coords.len();
    for j in 0..ns {
        let cp1 = section.coords[j];
        let cp2 = section.coords2.as_ref().map(|coords| coords[j]);
        let np = section.normals[j];
        for i in 0..centers.len() {
            let (cn, cb) = interpolated_coord(cp1, cp2, i, centers.len(), false);
            let binormal = tangents[i].cross(normals_3d[i]).normalize_or_zero();
            mesh.vertices
                .push(centers[i] + normals_3d[i] * cn + binormal * cb);
            mesh.normals
                .push((normals_3d[i] * np[0] + binormal * np[1]).normalize_or_zero());
        }
    }
    add_side_triangles(&mut mesh, ns, centers.len(), false);
    add_section_caps(
        &mut mesh,
        section,
        tangents,
        ns,
        centers.len(),
        cap_front,
        cap_back,
    );
    mesh
}

fn extrude_faceted(
    section: &SectionGeometry,
    centers: &[Vec3],
    tangents: &[Vec3],
    normals_3d: &[Vec3],
    cap_front: bool,
    cap_back: bool,
) -> MeshData {
    let mut mesh = MeshData::default();
    let ns = section.coords.len();
    let normals2 = section.normals2.as_ref().unwrap_or(&section.normals);
    for j in 0..ns {
        let cp1 = section.coords[j];
        let cp2 = section.coords2.as_ref().map(|coords| coords[j]);
        let np1 = section.normals[j];
        let np2 = normals2[j];
        for i in 0..centers.len() {
            let (cn, cb) = interpolated_coord(cp1, cp2, i, centers.len(), true);
            let binormal = tangents[i].cross(normals_3d[i]).normalize_or_zero();
            let pos = centers[i] + normals_3d[i] * cn + binormal * cb;
            mesh.vertices.push(pos);
            mesh.normals
                .push((normals_3d[i] * np1[0] + binormal * np1[1]).normalize_or_zero());
        }
        for i in 0..centers.len() {
            let (cn, cb) = interpolated_coord(cp1, cp2, i, centers.len(), true);
            let binormal = tangents[i].cross(normals_3d[i]).normalize_or_zero();
            let pos = centers[i] + normals_3d[i] * cn + binormal * cb;
            mesh.vertices.push(pos);
            mesh.normals
                .push((normals_3d[i] * np2[0] + binormal * np2[1]).normalize_or_zero());
        }
    }
    add_side_triangles(&mut mesh, ns, centers.len(), true);
    add_section_caps(
        &mut mesh,
        section,
        tangents,
        ns,
        centers.len(),
        cap_front,
        cap_back,
    );
    mesh
}

fn interpolated_coord(
    cp1: [f32; 2],
    cp2: Option<[f32; 2]>,
    i: usize,
    n: usize,
    faceted: bool,
) -> (f32, f32) {
    let Some(cp2) = cp2 else {
        return (cp1[0], cp1[1]);
    };
    let arrow_len = if faceted { n.saturating_sub(1) } else { n };
    let t = if arrow_len == 0 {
        1.0
    } else {
        (i.min(arrow_len) as f32) / arrow_len as f32
    };
    (
        cp1[0] * (1.0 - t) + cp2[0] * t,
        cp1[1] * (1.0 - t) + cp2[1] * t,
    )
}

fn add_side_triangles(mesh: &mut MeshData, ns: usize, np: usize, faceted: bool) {
    for s in 0..ns {
        let i_start = if faceted { s * 2 * np } else { s * np };
        let j = (s + 1) % ns;
        let j_start = if faceted { (j * 2 + 1) * np } else { j * np };
        for k in 0..(np - 1) {
            mesh.indices.extend_from_slice(&[
                (i_start + k + 1) as u32,
                (i_start + k) as u32,
                (j_start + k) as u32,
                (i_start + k + 1) as u32,
                (j_start + k) as u32,
                (j_start + k + 1) as u32,
            ]);
        }
    }
}

fn add_section_caps(
    mesh: &mut MeshData,
    section: &SectionGeometry,
    tangents: &[Vec3],
    ns: usize,
    np: usize,
    cap_front: bool,
    cap_back: bool,
) {
    if cap_front {
        let source_indices: Vec<usize> = if section.faceted {
            (0..ns).map(|i| i * 2 * np).collect()
        } else {
            (0..ns).map(|i| i * np).collect()
        };
        add_cap_from_existing(mesh, section, &source_indices, -tangents[0], true);
    }
    if cap_back {
        let last = np - 1;
        let source_indices: Vec<usize> = if section.faceted {
            (0..ns).map(|i| i * 2 * np + last).collect()
        } else {
            (0..ns).map(|i| i * np + last).collect()
        };
        add_cap_from_existing(mesh, section, &source_indices, tangents[last], false);
    }
}

fn add_cap_from_existing(
    mesh: &mut MeshData,
    section: &SectionGeometry,
    source_indices: &[usize],
    tangent: Vec3,
    reverse: bool,
) {
    let base = mesh.vertices.len() as u32;
    let tangent = tangent.normalize_or_zero();
    let cap_vertices: Vec<Vec3> = source_indices
        .iter()
        .map(|&index| mesh.vertices[index])
        .collect();
    for vertex in cap_vertices {
        mesh.vertices.push(vertex);
        mesh.normals.push(tangent);
    }
    for tri in &section.tessellation {
        if reverse {
            mesh.indices
                .extend_from_slice(&[base + tri[2], base + tri[1], base + tri[0]]);
        } else {
            mesh.indices
                .extend_from_slice(&[base + tri[0], base + tri[1], base + tri[2]]);
        }
    }
}

fn fallback_normal(tangent: Vec3) -> Vec3 {
    if tangent.dot(Vec3::Z).abs() < 0.98 {
        tangent.cross(Vec3::Z).normalize_or_zero()
    } else {
        tangent.cross(Vec3::X).normalize_or_zero()
    }
}

fn project_perpendicular(vector: Vec3, tangent: Vec3) -> Vec3 {
    vector - tangent * vector.dot(tangent)
}

impl Into<Shape> for Protein {
    fn into(self) -> Shape {
        Shape::Protein(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pdb_reads_backbone_with_cosmolkit() {
        let pdb = "\
ATOM      1  N   ALA A   1      11.104  13.207   9.900  1.00 20.00           N  
ATOM      2  CA  ALA A   1      12.210  13.912  10.555  1.00 20.00           C  
ATOM      3  C   ALA A   1      13.470  13.079  10.413  1.00 20.00           C  
ATOM      4  O   ALA A   1      14.000  12.500  11.000  1.00 20.00           O  
";

        let protein = Protein::from_pdb(pdb).expect("PDB backbone should parse");

        assert_eq!(protein.chains.len(), 1);
        assert_eq!(protein.chains[0].id, "A");
        assert_eq!(protein.chains[0].residues.len(), 1);
    }

    #[test]
    fn rainbow_residue_colors_match_chimerax_default_stops() {
        let colors = chimerax_rainbow_residue_colors(5, 1.0);

        assert_eq!(
            colors,
            vec![
                Vec4::new(0.0, 0.0, 1.0, 1.0),
                Vec4::new(0.0, 1.0, 1.0, 1.0),
                Vec4::new(0.0, 1.0, 0.0, 1.0),
                Vec4::new(1.0, 1.0, 0.0, 1.0),
                Vec4::new(1.0, 0.0, 0.0, 1.0),
            ]
        );
        assert_eq!(
            chimerax_rainbow_residue_colors(1, 0.5),
            vec![Vec4::new(0.0, 0.0, 1.0, 0.5)]
        );
    }

    #[test]
    fn obvious_backbone_gap_splits_ribbon_and_adds_dashes() {
        let pdb = "\
ATOM      1  N   ALA A   1      11.104  13.207   9.900  1.00 20.00           N  
ATOM      2  CA  ALA A   1      12.210  13.912  10.555  1.00 20.00           C  
ATOM      3  C   ALA A   1      13.470  13.079  10.413  1.00 20.00           C  
ATOM      4  O   ALA A   1      14.000  12.500  11.000  1.00 20.00           O  
ATOM      5  N   GLY A  10      31.104  13.207   9.900  1.00 20.00           N  
ATOM      6  CA  GLY A  10      32.210  13.912  10.555  1.00 20.00           C  
ATOM      7  C   GLY A  10      33.470  13.079  10.413  1.00 20.00           C  
ATOM      8  O   GLY A  10      34.000  12.500  11.000  1.00 20.00           O  
";
        let protein = Protein::from_pdb(pdb).expect("PDB backbone should parse");
        let residues: Vec<&Residue> = protein.chains[0].residues.iter().collect();

        assert_eq!(
            contiguous_backbone_ranges(&residues),
            vec![
                RibbonRange { start: 0, end: 1 },
                RibbonRange { start: 1, end: 2 },
            ]
        );

        let dashes = build_backbone_gap_dashes(&residues, None);
        assert!(!dashes.vertices.is_empty());
        assert!(dashes.colors.is_none());
    }

    #[test]
    fn from_mmcif_reads_atom_site_backbone_with_cosmolkit() {
        let cif = r#"
data_demo
loop_
_atom_site.group_PDB
_atom_site.id
_atom_site.type_symbol
_atom_site.label_atom_id
_atom_site.label_alt_id
_atom_site.label_comp_id
_atom_site.label_asym_id
_atom_site.label_seq_id
_atom_site.Cartn_x
_atom_site.Cartn_y
_atom_site.Cartn_z
_atom_site.occupancy
_atom_site.B_iso_or_equiv
_atom_site.auth_seq_id
_atom_site.auth_comp_id
_atom_site.auth_asym_id
_atom_site.auth_atom_id
_atom_site.pdbx_PDB_model_num
ATOM 1 N N . ALA A 1 11.104 13.207 9.900 1.00 20.00 1 ALA A N 1
ATOM 2 C CA . ALA A 1 12.210 13.912 10.555 1.00 20.00 1 ALA A CA 1
ATOM 3 C C . ALA A 1 13.470 13.079 10.413 1.00 20.00 1 ALA A C 1
ATOM 4 O O . ALA A 1 14.000 12.500 11.000 1.00 20.00 1 ALA A O 1
"#;

        let structure =
            read_mmcif_atom_site_subset_from_str(cif).expect("COSMolKit mmCIF should parse");
        let protein = Protein::from_biostructure(&structure).expect("BioStructure should convert");

        assert_eq!(protein.chains.len(), 1);
        assert_eq!(protein.chains[0].id, "A");
        assert_eq!(protein.chains[0].residues.len(), 1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SimdVertex {
    position: [f32; 3],
    normal: [f32; 3],
}
