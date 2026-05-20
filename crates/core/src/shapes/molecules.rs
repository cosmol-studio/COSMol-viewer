use crate::parser::sdf::Sdf;
use crate::parser::utils::BondType as SdfBondType;
use crate::utils::InstanceGroups;
pub use crate::utils::Logger;
use crate::{
    Shape,
    shapes::{sphere::SphereInstance, stick::StickInstance},
    utils::{Interaction, Interpolatable, IntoInstanceGroups, Material, MeshData, Stylable},
};
use cosmolkit::{
    BondOrder as CosmolkitBondOrder, Molecule as CosmolkitMolecule,
    io::sdf::{SdfCoordinateMode, read_sdf_from_str_with_coordinate_mode},
};
use glam::Vec3;
use na_seq::Element;
use serde::{Deserialize, Serialize};

pub fn my_color(element: &Element) -> Vec3 {
    // 优先使用自定义颜色
    match element {
        Element::Hydrogen => Vec3::new(1.0, 1.0, 1.0),
        Element::Carbon => Vec3::new(0.3, 0.3, 0.3),
        Element::Nitrogen => Vec3::new(0.2, 0.4, 1.0),
        Element::Oxygen => Vec3::new(1.0, 0.0, 0.0),
        Element::Fluorine => Vec3::new(0.0, 0.8, 0.0),
        Element::Phosphorus => Vec3::new(1.0, 0.5, 0.0),
        Element::Sulfur => Vec3::new(1.0, 1.0, 0.0),
        Element::Chlorine => Vec3::new(0.0, 0.8, 0.0),
        Element::Bromine => Vec3::new(0.6, 0.2, 0.2),
        Element::Iodine => Vec3::new(0.4, 0.0, 0.8),
        Element::Other => Vec3::new(0.8, 0.8, 0.8),
        _ => element.color().into(), // 其他未定义的元素
    }
}

pub fn my_radius(e: &Element) -> f32 {
    match e {
        Element::Hydrogen => 1.20,
        Element::Other => 1.20,
        _ => e.vdw_radius(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondType {
    SINGLE = 1,
    DOUBLE = 2,
    TRIPLE = 3,
    UNKNOWN = 4,
    AROMATIC = 0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoleculeStyle {
    BallAndStick,
    Stick,
    Sphere,
}

mod element_serde {
    use super::*;
    use serde::{Deserializer, Serializer, de::SeqAccess, ser::SerializeSeq};

    pub fn serialize<S>(elements: &Vec<Element>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(elements.len()))?;
        for elem in elements {
            let atomic_num: u8 = elem.atomic_number();
            seq.serialize_element(&atomic_num)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Element>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Vec<Element>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a sequence of atomic numbers")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut elements = Vec::new();
                while let Some(num) = seq.next_element::<u8>()? {
                    let elem = Element::from_atomic_number(num).unwrap_or(Element::Other);
                    elements.push(elem);
                }
                Ok(elements)
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Molecule {
    pub style: MoleculeStyle,
    #[serde(with = "element_serde")]
    pub atom_types: Vec<Element>,
    pub atom_colors: Option<Vec<Option<Vec3>>>,
    pub atom_posits: Vec<Vec3>,
    pub bond_types: Vec<BondType>,
    pub bond_indices: Vec<[usize; 2]>,
    pub quality: u32,

    pub visual_style: Material,
    pub interaction: Interaction,
}

impl Interpolatable for Molecule {
    fn interpolate(&self, other: &Self, t: f32, logger: impl Logger) -> Self {
        // check atom count
        if self.atom_posits.len() != other.atom_posits.len() {
            logger.error(format!(
                "Interpolation aborted: atom count differs (self: {}, other: {}). \
                Smooth interpolation requires scenes with identical atom structures.",
                self.atom_posits.len(),
                other.atom_posits.len()
            ));
            panic!("Smooth interpolation requires matching atom structures.");
        }

        // 检查键数量是否匹配（可选，根据需要）
        if self.bond_indices.len() != other.bond_indices.len() {
            logger.error(format!(
                "Interpolation aborted: bond topology differs (self: {}, other: {}). \
                Smooth interpolation cannot proceed with different bonding graphs.",
                self.bond_indices.len(),
                other.bond_indices.len()
            ));
            panic!("Smooth interpolation requires matching bond topology.");
        }

        // 原子坐标插值
        let atoms: Vec<Vec3> = self
            .atom_posits
            .iter()
            .zip(&other.atom_posits)
            .map(|(a, b)| {
                Vec3::new(
                    a[0] * (1.0 - t) + b[0] * t,
                    a[1] * (1.0 - t) + b[1] * t,
                    a[2] * (1.0 - t) + b[2] * t,
                )
            })
            .collect();

        let atom_colors: Option<Vec<Option<Vec3>>> =
            match (self.atom_colors.as_ref(), other.atom_colors.as_ref()) {
                (Some(colors_a), Some(colors_b)) => {
                    let colors: Vec<Option<Vec3>> = colors_a
                        .iter()
                        .enumerate()
                        .map(|(i, a)| {
                            let b = colors_b.get(i).and_then(|x| *x);

                            match (a, b) {
                                (Some(a), Some(b)) => Some(a * (1.0 - t) + b * t),

                                (None, Some(b)) => {
                                    let a_fallback = self.get_atom_colors(i);
                                    Some(a_fallback * (1.0 - t) + b * t)
                                }

                                (Some(a), None) => {
                                    let b_fallback = other.get_atom_colors(i);
                                    Some(a * (1.0 - t) + b_fallback * t)
                                }

                                (None, None) => None,
                            }
                        })
                        .collect();

                    Some(colors)
                }
                (None, Some(colors_b)) => Some(colors_b.clone()),
                (Some(colors_a), None) => Some(colors_a.clone()),
                (None, None) => None,
            };

        Self {
            style: self.style.clone(),
            atom_types: self.atom_types.clone(),
            atom_colors: atom_colors,
            atom_posits: atoms,
            bond_types: self.bond_types.clone(),
            bond_indices: self.bond_indices.clone(),
            quality: ((self.quality as f32) * (1.0 - t) + (other.quality as f32) * t) as u32,
            visual_style: self.visual_style.clone(),
            interaction: self.interaction.clone(),
        }
    }
}

impl Into<Shape> for Molecule {
    fn into(self) -> Shape {
        Shape::Molecules(self)
    }
}
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseSdfError {
    #[error("Failed to parse SDF data: '{0}'")]
    ParsingError(String),
}

impl Molecule {
    pub fn from_sdf(sdf: &str) -> Result<Self, ParseSdfError> {
        match Self::from_sdf_with_cosmolkit(sdf) {
            Ok(molecule) => Ok(molecule),
            Err(cosmolkit_error) => {
                eprintln!(
                    "[WARN] COSMolKit SDF parser failed; falling back to legacy SDF parser: {cosmolkit_error}"
                );
                let molecule_data = Sdf::new(sdf).map_err(|fallback_error| {
                    ParseSdfError::ParsingError(format!(
                        "COSMolKit parser failed: {cosmolkit_error}; fallback parser failed: {fallback_error}"
                    ))
                })?;
                Self::new(molecule_data)
            }
        }
    }

    fn from_sdf_with_cosmolkit(sdf: &str) -> Result<Self, ParseSdfError> {
        let record = read_sdf_from_str_with_coordinate_mode(sdf, SdfCoordinateMode::Require3D)
            .map_err(|e| ParseSdfError::ParsingError(e.to_string()))?;
        let mut molecule = record.molecule;
        for (field_name, field_value) in record.data_fields {
            molecule = molecule.with_sdf_data_field(field_name, field_value);
        }
        Self::from_cosmolkit(&molecule)
    }

    pub fn from_cosmolkit(molecule: &CosmolkitMolecule) -> Result<Self, ParseSdfError> {
        let molecule = match molecule.with_kekulized_bonds(true) {
            Ok(molecule) => molecule,
            Err(error) => {
                eprintln!(
                    "[WARN] COSMolKit kekulize failed; keeping original aromatic bond representation: {error}"
                );
                molecule.clone()
            }
        };

        let molecule = if molecule.conformers_3d().is_empty() && molecule.coords_2d().is_none() {
            molecule
                .with_2d_coordinates()
                .map_err(|e| ParseSdfError::ParsingError(e.to_string()))?
        } else {
            molecule
        };

        let atom_posits = if let Some(conformer) = molecule.conformers_3d().first() {
            let coords = conformer.coords();
            let atom_count = molecule.atoms().len();
            if coords.len() < atom_count {
                return Err(ParseSdfError::ParsingError(format!(
                    "COSMolKit atom/coordinate count mismatch: {} atoms, {} coordinates",
                    atom_count,
                    coords.len()
                )));
            }
            if coords.len() > atom_count {
                eprintln!(
                    "[WARN] COSMolKit exposed {} coordinates for {atom_count} atoms; ignoring trailing coordinates",
                    coords.len()
                );
            }
            coords
                .iter()
                .take(atom_count)
                .map(|coord| Vec3::new(coord[0] as f32, coord[1] as f32, coord[2] as f32))
                .collect()
        } else if let Some(coords) = molecule.coords_2d() {
            let atom_count = molecule.atoms().len();
            if coords.len() < atom_count {
                return Err(ParseSdfError::ParsingError(format!(
                    "COSMolKit atom/coordinate count mismatch: {} atoms, {} coordinates",
                    atom_count,
                    coords.len()
                )));
            }
            if coords.len() > atom_count {
                eprintln!(
                    "[WARN] COSMolKit exposed {} coordinates for {atom_count} atoms; ignoring trailing coordinates",
                    coords.len()
                );
            }
            coords
                .iter()
                .take(atom_count)
                .map(|coord| Vec3::new(coord[0] as f32, coord[1] as f32, 0.0))
                .collect()
        } else {
            return Err(ParseSdfError::ParsingError(
                "COSMolKit did not expose coordinates".to_string(),
            ));
        };

        let atom_types = molecule
            .atoms()
            .iter()
            .map(|atom| Element::from_atomic_number(atom.atomic_number()).unwrap_or(Element::Other))
            .collect();
        let atom_colors = atom_colors_from_cosmolkit_weights(&molecule);

        let mut bond_indices = Vec::with_capacity(molecule.bonds().len());
        let mut bond_types = Vec::with_capacity(molecule.bonds().len());
        for bond in molecule.bonds() {
            let begin_atom = bond.begin().index();
            let end_atom = bond.end().index();
            if begin_atom >= molecule.atoms().len() || end_atom >= molecule.atoms().len() {
                return Err(ParseSdfError::ParsingError(format!(
                    "COSMolKit bond {} references out-of-range atoms {}-{}",
                    bond.id().index(),
                    begin_atom,
                    end_atom
                )));
            }
            bond_indices.push([begin_atom, end_atom]);
            bond_types.push(match bond.order() {
                CosmolkitBondOrder::Single => BondType::SINGLE,
                CosmolkitBondOrder::Double => BondType::DOUBLE,
                CosmolkitBondOrder::Triple => BondType::TRIPLE,
                CosmolkitBondOrder::Aromatic if bond.is_aromatic() => BondType::AROMATIC,
                CosmolkitBondOrder::Aromatic => BondType::AROMATIC,
                _ => BondType::UNKNOWN,
            });
        }

        Ok(Self {
            style: MoleculeStyle::BallAndStick,
            atom_types,
            atom_posits,
            atom_colors,
            bond_types,
            bond_indices,
            quality: 6,
            visual_style: Material {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
            interaction: Default::default(),
        })
    }

    pub fn from_atom_bond_data(
        atom_atomic_numbers: Vec<u8>,
        atom_posits: Vec<[f32; 3]>,
        bond_order_codes: Vec<i64>,
        bond_indices: Vec<[usize; 2]>,
        atom_colors: Option<Vec<Option<Vec3>>>,
    ) -> Result<Self, ParseSdfError> {
        if atom_atomic_numbers.len() != atom_posits.len() {
            return Err(ParseSdfError::ParsingError(format!(
                "atom/coordinate count mismatch: {} atoms, {} coordinates",
                atom_atomic_numbers.len(),
                atom_posits.len()
            )));
        }
        if bond_order_codes.len() != bond_indices.len() {
            return Err(ParseSdfError::ParsingError(format!(
                "bond order/index count mismatch: {} bond orders, {} bond indices",
                bond_order_codes.len(),
                bond_indices.len()
            )));
        }
        if let Some(colors) = atom_colors.as_ref()
            && colors.len() != atom_atomic_numbers.len()
        {
            return Err(ParseSdfError::ParsingError(format!(
                "atom/color count mismatch: {} atoms, {} colors",
                atom_atomic_numbers.len(),
                colors.len()
            )));
        }

        let atom_count = atom_atomic_numbers.len();
        for [begin, end] in &bond_indices {
            if *begin >= atom_count || *end >= atom_count {
                return Err(ParseSdfError::ParsingError(format!(
                    "bond references out-of-range atoms {begin}-{end}; atom count is {atom_count}"
                )));
            }
        }

        let atom_types = atom_atomic_numbers
            .into_iter()
            .map(|atomic_num| Element::from_atomic_number(atomic_num).unwrap_or(Element::Other))
            .collect();
        let atom_posits = atom_posits
            .into_iter()
            .map(|posit| Vec3::new(posit[0], posit[1], posit[2]))
            .collect();
        let bond_types = bond_order_codes
            .into_iter()
            .map(|code| match code {
                1 => BondType::SINGLE,
                2 => BondType::DOUBLE,
                3 => BondType::TRIPLE,
                5 => BondType::AROMATIC,
                _ => BondType::UNKNOWN,
            })
            .collect();

        Ok(Self {
            style: MoleculeStyle::BallAndStick,
            atom_types,
            atom_posits,
            atom_colors,
            bond_types,
            bond_indices,
            quality: 6,
            visual_style: Material {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
            interaction: Default::default(),
        })
    }

    fn new(sdf: Sdf) -> Result<Self, ParseSdfError> {
        // Split atoms into positions + types in one pass
        let (atom_posits, atom_types): (Vec<Vec3>, Vec<Element>) = sdf
            .atoms
            .into_iter()
            .map(|atom| (atom.posit, atom.element))
            .unzip();

        let atom_colors = match sdf.atoms_weight {
            Some(weights) => {
                let mut atom_colors = Vec::new();
                for weight_opt in weights {
                    if let Some(weight) = weight_opt {
                        atom_colors.push(Some(Vec3::new(weight, 1.0 - weight, 0.0)));
                    } else {
                        atom_colors.push(None);
                    }
                }
                Some(atom_colors)
            }
            None => None,
        };

        // Split bonds into indices + types in one pass
        let (bond_indices, bond_types): (Vec<[usize; 2]>, Vec<BondType>) = sdf
            .bonds
            .into_iter()
            .map(|bond| {
                let indices = [bond.atom_0_sn as usize - 1, bond.atom_1_sn as usize - 1];

                let bond_type = match bond.bond_type {
                    SdfBondType::Single => BondType::SINGLE,
                    SdfBondType::Double => BondType::DOUBLE,
                    SdfBondType::Triple => BondType::TRIPLE,
                    SdfBondType::Aromatic => BondType::AROMATIC,
                    _ => BondType::UNKNOWN,
                };

                (indices, bond_type)
            })
            .unzip();

        Ok(Self {
            style: MoleculeStyle::BallAndStick,
            atom_types,
            atom_posits,
            atom_colors,
            bond_types,
            bond_indices,
            quality: 6,
            visual_style: Material {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
            interaction: Default::default(),
        })
    }

    pub fn get_center(&self) -> [f32; 3] {
        if self.atom_posits.is_empty() {
            return [0.0; 3];
        }

        // 1. 累加所有原子坐标
        let mut center = [0.0f32; 3];
        for pos in &self.atom_posits {
            center[0] += pos[0];
            center[1] += pos[1];
            center[2] += pos[2];
        }

        // 2. 计算平均值
        let count = self.atom_posits.len() as f32;
        center[0] /= count;
        center[1] /= count;
        center[2] /= count;

        center
    }

    /// Centers the molecule by translating all atoms so that the geometric center
    /// is at the origin (0.0, 0.0, 0.0).
    pub fn centered(mut self) -> Self {
        let center = self.get_center();
        for atom in &mut self.atom_posits {
            atom[0] -= center[0];
            atom[1] -= center[1];
            atom[2] -= center[2];
        }

        self
    }

    pub fn reset_color(mut self) -> Self {
        self.style_mut().color = None;
        self
    }

    pub fn to_mesh(&self, _scale: f32) -> MeshData {
        MeshData::default()
    }

    pub fn get_atom_colors(&self, index: usize) -> Vec3 {
        if let Some(colors) = &self.atom_colors {
            if let Some(Some(c)) = colors.get(index) {
                c.clone()
            } else {
                self.visual_style
                    .color
                    .unwrap_or_else(|| self.atom_types.get(index).map(my_color).unwrap())
            }
        } else {
            // atom_colors 整体是 None → fallback
            self.visual_style
                .color
                .unwrap_or_else(|| self.atom_types.get(index).map(my_color).unwrap())
        }
    }

    fn first_atom_neighbors(&self) -> Vec<[usize; 2]> {
        let mut neighbors = vec![[usize::MAX; 2]; self.atom_posits.len()];

        for [a, b] in &self.bond_indices {
            if *a < neighbors.len() && *b < neighbors.len() {
                add_first_neighbor(&mut neighbors[*a], *b);
                add_first_neighbor(&mut neighbors[*b], *a);
            }
        }

        neighbors
    }

    fn get_bond_atom_color(&self, index: usize) -> Vec3 {
        self.visual_style.color.unwrap_or_else(|| {
            self.atom_types
                .get(index)
                .map(|x| match x {
                    Element::Carbon => Vec3::new(0.75, 0.75, 0.75),
                    _ => my_color(x),
                })
                .unwrap()
        })
    }
}

fn add_first_neighbor(neighbors: &mut [usize; 2], atom: usize) {
    if neighbors.contains(&atom) {
        return;
    }

    if neighbors[0] == usize::MAX {
        neighbors[0] = atom;
    } else if neighbors[1] == usize::MAX {
        neighbors[1] = atom;
    }
}

fn first_neighbor_except(neighbors: [usize; 2], excluded: usize) -> Option<usize> {
    neighbors
        .into_iter()
        .find(|neighbor| *neighbor != usize::MAX && *neighbor != excluded)
}

fn scaled_point(point: [f32; 3], scale: f32) -> [f32; 3] {
    [point[0] * scale, point[1] * scale, point[2] * scale]
}

impl IntoInstanceGroups for Molecule {
    fn to_instance_group(&self, scale: f32) -> InstanceGroups {
        let mut groups = InstanceGroups {
            spheres: Vec::with_capacity(self.atom_posits.len()),
            sticks: Vec::with_capacity(self.bond_indices.len() * 6),
        };

        let first_neighbors = self.first_atom_neighbors();
        let alpha = self.visual_style.opacity.clamp(0.0, 1.0);
        let material = [Material::default().roughness, Material::default().metallic];

        for (i, pos) in self.atom_posits.iter().enumerate() {
            let color = self.get_atom_colors(i);
            groups.spheres.push(SphereInstance::new(
                [pos[0] * scale, pos[1] * scale, pos[2] * scale],
                self.atom_types.get(i).map(|x| my_radius(x) * 0.2).unwrap() * scale,
                [color[0], color[1], color[2], alpha],
                material,
            ));
        }

        for (i, bond) in self.bond_indices.iter().enumerate() {
            let [a, b] = bond;
            let pos_a = self.atom_posits[*a];
            let pos_b = self.atom_posits[*b];

            let bond_type = self.bond_types.get(i).unwrap_or(&BondType::SINGLE);

            // 方向向量
            let dir = [
                pos_b[0] - pos_a[0],
                pos_b[1] - pos_a[1],
                pos_b[2] - pos_a[2],
            ];

            // 归一化方向
            let norm = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
            let dir_n = [dir[0] / norm, dir[1] / norm, dir[2] / norm];

            // === Step 1: 先找 A 的邻居方向（排除 B）===
            let mut neighbor_dir_opt =
                first_neighbor_except(first_neighbors[*a], *b).map(|neighbor| {
                    let pos_n = self.atom_posits[neighbor];
                    [
                        pos_n[0] - pos_a[0],
                        pos_n[1] - pos_a[1],
                        pos_n[2] - pos_a[2],
                    ]
                });

            // ✅ 若 A 没有邻居，则去找 B 的邻居
            if neighbor_dir_opt.is_none() {
                neighbor_dir_opt = first_neighbor_except(first_neighbors[*b], *a).map(|neighbor| {
                    let pos_n = self.atom_posits[neighbor];
                    [
                        pos_n[0] - pos_b[0],
                        pos_n[1] - pos_b[1],
                        pos_n[2] - pos_b[2],
                    ]
                });
            }

            // === Step 2: 计算 offset 方向 ===
            let offset = if let Some(nd) = neighbor_dir_opt {
                // 用邻居方向构造共面偏移
                let nd_norm = (nd[0] * nd[0] + nd[1] * nd[1] + nd[2] * nd[2]).sqrt();
                let nd_n = [nd[0] / nd_norm, nd[1] / nd_norm, nd[2] / nd_norm];

                // 计算 nd_n 在 dir_n 方向的投影分量
                let dot = nd_n[0] * dir_n[0] + nd_n[1] * dir_n[1] + nd_n[2] * dir_n[2];
                let proj = [dot * dir_n[0], dot * dir_n[1], dot * dir_n[2]];

                // 去掉投影分量，得到“共面但不沿键方向”的偏移矢量
                [nd_n[0] - proj[0], nd_n[1] - proj[1], nd_n[2] - proj[2]]
            } else {
                // ✅ A 和 B 都没有邻居 → 回到默认垂直方向
                let up = if dir_n[0].abs() < 0.9 {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 1.0, 0.0]
                };
                [
                    dir_n[1] * up[2] - dir_n[2] * up[1],
                    dir_n[2] * up[0] - dir_n[0] * up[2],
                    dir_n[0] * up[1] - dir_n[1] * up[0],
                ]
            };

            // 归一化 offset
            let off_norm =
                (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
            let off_n = [
                offset[0] / off_norm,
                offset[1] / off_norm,
                offset[2] / off_norm,
            ];

            // 颜色和半径与原来一致
            let color_a = self.get_bond_atom_color(*a);
            let color_b = self.get_bond_atom_color(*b);
            let color_a = [color_a[0], color_a[1], color_a[2], alpha];
            let color_b = [color_b[0], color_b[1], color_b[2], alpha];

            // 根据键类型生成多个 stick
            let (num_sticks, radius) = match bond_type {
                BondType::SINGLE => (1, 0.135),
                BondType::DOUBLE => (2, 0.09),
                BondType::TRIPLE => (3, 0.05),
                BondType::AROMATIC => (2, 0.09),
                _ => (1, 0.05), // aromatic等以后再处理
            };
            let offset_step = match bond_type {
                BondType::TRIPLE => 0.14,
                _ => 0.22,
            };

            for k in 0..num_sticks {
                let offset_mul = (k as f32 - (num_sticks - 1) as f32 * 0.5) * offset_step;

                let pos_a_k = [
                    pos_a[0] + off_n[0] * offset_mul,
                    pos_a[1] + off_n[1] * offset_mul,
                    pos_a[2] + off_n[2] * offset_mul,
                ];
                let pos_b_k = [
                    pos_b[0] + off_n[0] * offset_mul,
                    pos_b[1] + off_n[1] * offset_mul,
                    pos_b[2] + off_n[2] * offset_mul,
                ];

                let midpoint = [
                    0.5 * (pos_a_k[0] + pos_b_k[0]),
                    0.5 * (pos_a_k[1] + pos_b_k[1]),
                    0.5 * (pos_a_k[2] + pos_b_k[2]),
                ];

                groups.sticks.push(StickInstance::new(
                    scaled_point(pos_a_k, scale),
                    scaled_point(midpoint, scale),
                    radius * scale,
                    color_a,
                    material,
                ));

                groups.sticks.push(StickInstance::new(
                    scaled_point(pos_b_k, scale),
                    scaled_point(midpoint, scale),
                    radius * scale,
                    color_b,
                    material,
                ));
            }
        }
        groups
    }
}

impl Stylable for Molecule {
    fn style_mut(&mut self) -> &mut Material {
        &mut self.visual_style
    }
}

fn color_from_weight(weight: f64) -> Vec3 {
    Vec3::new(weight as f32, 1.0 - weight as f32, 0.0)
}

fn parse_f64_lines(value: &str) -> Option<Vec<f64>> {
    value
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn parse_usize_lines(value: &str) -> Option<Vec<usize>> {
    value
        .split_whitespace()
        .map(str::parse::<usize>)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

fn sdf_data_field<'a>(molecule: &'a CosmolkitMolecule, name: &str) -> Option<&'a str> {
    molecule
        .properties()
        .sdf_data_fields()
        .iter()
        .rev()
        .find(|(field_name, _)| field_name.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn parse_dummy_atom_weights(
    indices_value: Option<&str>,
    weights_value: &str,
    atom_count: usize,
) -> Option<Vec<Option<Vec3>>> {
    let weights = parse_f64_lines(weights_value)?;

    if let Some(indices_value) = indices_value {
        let indices = parse_usize_lines(indices_value)?;
        if indices.len() != weights.len() {
            eprintln!(
                "[WARN] Ignoring DUMMY_ATOM_INDICES/DUMMY_WEIGHTS: expected matching lengths, got {} indices and {} weights",
                indices.len(),
                weights.len()
            );
            return None;
        }

        let mut colors = vec![None; atom_count];
        for (atom_index, weight) in indices.into_iter().zip(weights) {
            if atom_index == 0 || atom_index > atom_count {
                eprintln!(
                    "[WARN] Ignoring DUMMY_ATOM_INDICES/DUMMY_WEIGHTS: atom index {atom_index} out of range 1..={atom_count}"
                );
                return None;
            }
            colors[atom_index - 1] = Some(color_from_weight(weight));
        }

        return Some(colors);
    }

    if weights.len() != atom_count {
        eprintln!(
            "[WARN] Ignoring DUMMY_WEIGHTS: expected {atom_count} values for all atoms, got {}",
            weights.len()
        );
        return None;
    }

    Some(
        weights
            .into_iter()
            .map(|weight| Some(color_from_weight(weight)))
            .collect(),
    )
}

fn atom_colors_from_cosmolkit_weights(molecule: &CosmolkitMolecule) -> Option<Vec<Option<Vec3>>> {
    if let Some(colors) = sdf_data_field(molecule, "DUMMY_WEIGHTS").and_then(|weights_value| {
        parse_dummy_atom_weights(
            sdf_data_field(molecule, "DUMMY_ATOM_INDICES"),
            weights_value,
            molecule.atoms().len(),
        )
    }) {
        return Some(colors);
    }

    let atom_colors = molecule
        .atoms()
        .iter()
        .map(|atom| {
            atom.prop("WEIGHT")
                .and_then(|value| value.parse::<f64>().ok())
                .map(color_from_weight)
        })
        .collect::<Vec<_>>();

    if atom_colors.iter().any(Option::is_some) {
        Some(atom_colors)
    } else {
        None
    }
}

impl TryFrom<&CosmolkitMolecule> for Molecule {
    type Error = ParseSdfError;

    fn try_from(molecule: &CosmolkitMolecule) -> Result<Self, Self::Error> {
        Self::from_cosmolkit(molecule)
    }
}

impl TryFrom<CosmolkitMolecule> for Molecule {
    type Error = ParseSdfError;

    fn try_from(molecule: CosmolkitMolecule) -> Result<Self, Self::Error> {
        Self::from_cosmolkit(&molecule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_sdf_kekulizes_aromatic_bonds() {
        let sdf = "\
benzene
  cosmol_viewer

  6  6  0  0  0  0  0  0  0  0999 V2000
    1.3960    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.6980    1.2090    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -0.6980    1.2090    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -1.3960    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
   -0.6980   -1.2090    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.6980   -1.2090    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  4  0  0  0  0
  2  3  4  0  0  0  0
  3  4  4  0  0  0  0
  4  5  4  0  0  0  0
  5  6  4  0  0  0  0
  6  1  4  0  0  0  0
M  END
$$$$
";

        let molecule = Molecule::from_sdf(sdf).expect("benzene SDF should parse");
        let single_count = molecule
            .bond_types
            .iter()
            .filter(|bond_type| **bond_type == BondType::SINGLE)
            .count();
        let double_count = molecule
            .bond_types
            .iter()
            .filter(|bond_type| **bond_type == BondType::DOUBLE)
            .count();

        assert_eq!(single_count, 3);
        assert_eq!(double_count, 3);
    }

    #[test]
    fn from_cosmolkit_molecule_generates_coords_and_kekulizes() {
        let cosmolkit_molecule =
            CosmolkitMolecule::from_smiles("c1ccccc1").expect("SMILES should parse");
        let molecule = Molecule::from_cosmolkit(&cosmolkit_molecule)
            .expect("COSMolKit molecule should convert");

        assert_eq!(molecule.atom_types.len(), 6);
        assert_eq!(molecule.atom_posits.len(), 6);
        assert_eq!(molecule.bond_types.len(), 6);
        assert!(
            molecule
                .atom_posits
                .iter()
                .any(|posit| posit.length() > 0.0)
        );

        let single_count = molecule
            .bond_types
            .iter()
            .filter(|bond_type| **bond_type == BondType::SINGLE)
            .count();
        let double_count = molecule
            .bond_types
            .iter()
            .filter(|bond_type| **bond_type == BondType::DOUBLE)
            .count();

        assert_eq!(single_count, 3);
        assert_eq!(double_count, 3);
    }

    #[test]
    fn from_sdf_reads_dummy_weights_data_field() {
        let sdf = "\
DummyPoints
  COSMolKit

  0  0  0     0  0            999 V3000
M  V30 BEGIN CTAB
M  V30 COUNTS 2 0 0 0 0
M  V30 BEGIN ATOM
M  V30 1 * 1.0 2.0 3.0 0
M  V30 2 * 4.0 5.0 6.0 0
M  V30 END ATOM
M  V30 END CTAB
M  END
>  <DUMMY_WEIGHTS>
0.25
0.75

$$$$
";

        let record = read_sdf_from_str_with_coordinate_mode(sdf, SdfCoordinateMode::Require3D)
            .expect("COSMolKit should parse dummy point SDF");
        assert_eq!(
            record.data_fields,
            vec![("DUMMY_WEIGHTS".to_string(), "0.25\n0.75".to_string())]
        );

        let molecule = Molecule::from_sdf(sdf).expect("dummy point SDF should convert");

        assert_eq!(molecule.atom_types, vec![Element::Other, Element::Other]);
        assert_eq!(molecule.bond_types.len(), 0);
        assert_eq!(
            molecule.atom_colors,
            Some(vec![
                Some(Vec3::new(0.25, 0.75, 0.0)),
                Some(Vec3::new(0.75, 0.25, 0.0)),
            ])
        );
    }

    #[test]
    fn from_sdf_reads_indexed_dummy_weights_data_fields() {
        let sdf = "\
GraphWithDummyPoints
  COSMolKit

  0  0  0     0  0            999 V3000
M  V30 BEGIN CTAB
M  V30 COUNTS 4 2 0 0 0
M  V30 BEGIN ATOM
M  V30 1 C 0.0 0.0 0.0 0
M  V30 2 O 1.0 0.0 0.0 0
M  V30 3 * 2.0 0.0 0.0 0
M  V30 4 * 3.0 0.0 0.0 0
M  V30 END ATOM
M  V30 BEGIN BOND
M  V30 1 1 1 2
M  V30 2 1 2 3
M  V30 END BOND
M  V30 END CTAB
M  END
>  <DUMMY_ATOM_INDICES>
3
4

>  <DUMMY_WEIGHTS>
0.25
0.75

$$$$
";

        let record = read_sdf_from_str_with_coordinate_mode(sdf, SdfCoordinateMode::Require3D)
            .expect("COSMolKit should parse indexed dummy point SDF");
        assert_eq!(
            record.data_fields,
            vec![
                ("DUMMY_ATOM_INDICES".to_string(), "3\n4".to_string()),
                ("DUMMY_WEIGHTS".to_string(), "0.25\n0.75".to_string()),
            ]
        );

        let molecule = Molecule::from_sdf(sdf).expect("indexed dummy point SDF should convert");

        assert_eq!(
            molecule.atom_types,
            vec![
                Element::Carbon,
                Element::Oxygen,
                Element::Other,
                Element::Other
            ]
        );
        assert_eq!(
            molecule.bond_types,
            vec![BondType::SINGLE, BondType::SINGLE]
        );
        assert_eq!(
            molecule.atom_colors,
            Some(vec![
                None,
                None,
                Some(Vec3::new(0.25, 0.75, 0.0)),
                Some(Vec3::new(0.75, 0.25, 0.0)),
            ])
        );
    }
}
