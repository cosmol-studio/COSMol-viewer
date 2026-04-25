use crate::parser::sdf::Sdf;
use crate::parser::utils::BondType as SdfBondType;
use crate::utils::InstanceGroups;
pub use crate::utils::Logger;
use crate::{
    Shape,
    shapes::{sphere::Sphere, stick::Stick},
    utils::{Interaction, Interpolatable, IntoInstanceGroups, Material, MeshData, Stylable},
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
        let molecule_data =
            Sdf::new(sdf).map_err(|e| ParseSdfError::ParsingError(e.to_string()))?;
        Self::new(molecule_data)
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
}

impl IntoInstanceGroups for Molecule {
    fn to_instance_group(&self, scale: f32) -> InstanceGroups {
        let mut groups = InstanceGroups::default();

        for (i, pos) in self.atom_posits.iter().enumerate() {
            let sphere_instance = Sphere::new(
                pos.to_array(),
                self.atom_types.get(i).map(|x| my_radius(x) * 0.2).unwrap(),
            )
            .color(self.get_atom_colors(i))
            .opacity(self.visual_style.opacity);

            groups.spheres.push(sphere_instance.to_instance(scale));
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
            let mut neighbor_dir_opt = None;
            for (_j, other_bond) in self.bond_indices.iter().enumerate() {
                let [x, y] = other_bond;
                if x == a && y != b {
                    let pos_n = self.atom_posits[*y];
                    neighbor_dir_opt = Some([
                        pos_n[0] - pos_a[0],
                        pos_n[1] - pos_a[1],
                        pos_n[2] - pos_a[2],
                    ]);
                    break;
                } else if y == a && x != b {
                    let pos_n = self.atom_posits[*x];
                    neighbor_dir_opt = Some([
                        pos_n[0] - pos_a[0],
                        pos_n[1] - pos_a[1],
                        pos_n[2] - pos_a[2],
                    ]);
                    break;
                }
            }

            // ✅ 若 A 没有邻居，则去找 B 的邻居
            if neighbor_dir_opt.is_none() {
                for (_j, other_bond) in self.bond_indices.iter().enumerate() {
                    let [x, y] = other_bond;
                    if x == b && y != a {
                        let pos_n = self.atom_posits[*y];
                        neighbor_dir_opt = Some([
                            pos_n[0] - pos_b[0],
                            pos_n[1] - pos_b[1],
                            pos_n[2] - pos_b[2],
                        ]);
                        break;
                    } else if y == b && x != a {
                        let pos_n = self.atom_posits[*x];
                        neighbor_dir_opt = Some([
                            pos_n[0] - pos_b[0],
                            pos_n[1] - pos_b[1],
                            pos_n[2] - pos_b[2],
                        ]);
                        break;
                    }
                }
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
            let color_a = self.visual_style.color.unwrap_or(
                self.atom_types
                    .get(*a)
                    .map(|x| match x {
                        Element::Carbon => Vec3::new(0.75, 0.75, 0.75),
                        _ => my_color(x),
                    })
                    .unwrap(),
            );
            let color_b = self.visual_style.color.unwrap_or(
                self.atom_types
                    .get(*b)
                    .map(|x| match x {
                        Element::Carbon => Vec3::new(0.75, 0.75, 0.75),
                        _ => my_color(x),
                    })
                    .unwrap(),
            );

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

                // A -> 中点
                let stick_a = Stick::new(
                    pos_a_k,
                    [
                        0.5 * (pos_a_k[0] + pos_b_k[0]),
                        0.5 * (pos_a_k[1] + pos_b_k[1]),
                        0.5 * (pos_a_k[2] + pos_b_k[2]),
                    ],
                    radius,
                )
                .color(color_a)
                .opacity(self.visual_style.opacity);

                groups.sticks.push(stick_a.to_instance(scale));

                // B -> 中点
                let stick_b = Stick::new(
                    pos_b_k,
                    [
                        0.5 * (pos_a_k[0] + pos_b_k[0]),
                        0.5 * (pos_a_k[1] + pos_b_k[1]),
                        0.5 * (pos_a_k[2] + pos_b_k[2]),
                    ],
                    radius,
                )
                .color(color_b)
                .opacity(self.visual_style.opacity);

                groups.sticks.push(stick_b.to_instance(scale));
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
