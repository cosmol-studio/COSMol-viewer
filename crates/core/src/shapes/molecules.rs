pub use crate::utils::Logger;
use std::collections::VecDeque;

use crate::utils::{Color, InstanceGroups, OutlineInstanceGroup, OutlineSettings};
use crate::{
    Shape,
    shapes::{sphere::SphereInstance, stick::StickInstance},
    utils::{Interaction, Interpolatable, IntoInstanceGroups, Material, MeshData, Stylable},
};
use cosmolkit::{
    BondOrder as CosmolkitBondOrder, Element, Molecule as CosmolkitMolecule,
    io::sdf::{SdfCoordinateMode, SdfReadParams, read_sdf_from_str_with_params},
};
use glam::Vec3;
use serde::{Deserialize, Serialize};

pub fn my_color(element: &Element) -> Vec3 {
    match element.atomic_number() {
        1 => Vec3::new(1.0, 1.0, 1.0),
        6 => Vec3::new(0.3, 0.3, 0.3),
        7 => Vec3::new(0.2, 0.4, 1.0),
        8 => Vec3::new(1.0, 0.0, 0.0),
        9 | 17 => Vec3::new(0.0, 0.8, 0.0),
        15 => Vec3::new(1.0, 0.5, 0.0),
        16 => Vec3::new(1.0, 1.0, 0.0),
        35 => Vec3::new(0.6, 0.2, 0.2),
        53 => Vec3::new(0.4, 0.0, 0.8),
        _ => Vec3::new(0.8, 0.8, 0.8),
    }
}

pub fn my_radius(e: &Element) -> f32 {
    match e.atomic_number() {
        1 => 1.20,
        6 => 1.70,
        7 => 1.55,
        8 => 1.52,
        9 => 1.47,
        15 => 1.80,
        16 => 1.80,
        17 => 1.75,
        35 => 1.85,
        53 => 1.98,
        _ => 1.20,
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
                    let elem = Element::from_atomic_number(num).unwrap_or(Element::DUMMY);
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
    #[serde(default)]
    pub outline: OutlineSettings,
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
            outline: self.outline,
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
        let record = read_sdf_from_str_with_params(
            sdf,
            SdfReadParams {
                sanitize: false,
                remove_hs: false,
                coordinate_mode: SdfCoordinateMode::Require3D,
                ..Default::default()
            },
        )
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

        let molecule = if molecule.conformers_3d().is_empty() && molecule.coordinates_2d().is_none()
        {
            molecule
                .with_2d_coordinates()
                .map_err(|e| ParseSdfError::ParsingError(e.to_string()))?
        } else {
            molecule
        };

        let atom_posits = if let Some(conformer) = molecule.conformers_3d().first() {
            let coords = conformer.coordinates();
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
        } else if let Some(coords) = molecule.coordinates_2d() {
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
            .map(|atom| Element::from_atomic_number(atom.atomic_number()).unwrap_or(Element::DUMMY))
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
            outline: OutlineSettings::default(),
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
            .map(|atomic_num| Element::from_atomic_number(atomic_num).unwrap_or(Element::DUMMY))
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
            outline: OutlineSettings::default(),
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

    pub fn set_outline<C: Into<Color>>(mut self, enabled: bool, color: C, width: f32) -> Self {
        self.outline = OutlineSettings {
            enabled,
            color: color.into().into(),
            width: width.max(0.0),
        };
        self
    }

    pub fn enable_outline(mut self, width: f32) -> Self {
        self.outline.enabled = true;
        self.outline.width = width.max(0.0);
        self
    }

    pub fn disable_outline(mut self) -> Self {
        self.outline.enabled = false;
        self
    }

    /// Display atoms as spheres and bonds as sticks. This is the default style.
    pub fn ball_and_stick(mut self) -> Self {
        self.style = MoleculeStyle::BallAndStick;
        self
    }

    /// Display bonds as ChimeraX-style sticks with same-radius endpoint spheres for smooth joins.
    ///
    /// Double and triple bonds remain visible, using thinner parallel sticks with spacing derived
    /// from the ChimeraX stick radius. Aromatic ring bonds render as a single stick plus an inner
    /// aromatic line.
    pub fn stick(mut self) -> Self {
        self.style = MoleculeStyle::Stick;
        self
    }

    /// Display only atoms as spheres, without bonds.
    pub fn sphere(mut self) -> Self {
        self.style = MoleculeStyle::Sphere;
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

    fn get_bond_atom_color(&self, index: usize) -> Vec3 {
        self.visual_style.color.unwrap_or_else(|| {
            self.atom_types
                .get(index)
                .map(|x| match x {
                    element if *element == Element::C => Vec3::new(0.75, 0.75, 0.75),
                    _ => my_color(x),
                })
                .unwrap()
        })
    }
}

fn scaled_point(point: [f32; 3], scale: f32) -> [f32; 3] {
    [point[0] * scale, point[1] * scale, point[2] * scale]
}

const BOND_EPSILON: f32 = 1.0e-6;
const CHIMERAX_STICK_RADIUS: f32 = 0.2;
const CHIMERAX_MULTIBOND_GAP: f32 = 0.02;
const AROMATIC_LINE_RADIUS_SCALE: f32 = 0.38;
const AROMATIC_LINE_GAP: f32 = 0.02;

fn perpendicular_unit_vector(dir: Vec3) -> Vec3 {
    let up = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    dir.cross(up).normalize()
}

fn bond_offset_direction(
    atom_index: usize,
    excluded_atom: usize,
    atom_posits: &[Vec3],
    bond_indices: &[[usize; 2]],
    bond_dir: Vec3,
) -> Option<Vec3> {
    bond_indices
        .iter()
        .filter_map(|[a, b]| {
            let neighbor = if *a == atom_index && *b != excluded_atom {
                Some(*b)
            } else if *b == atom_index && *a != excluded_atom {
                Some(*a)
            } else {
                None
            }?;

            let neighbor_dir = atom_posits[neighbor] - atom_posits[atom_index];
            let neighbor_dir = neighbor_dir.try_normalize()?;
            let projected = neighbor_dir - neighbor_dir.dot(bond_dir) * bond_dir;
            projected.try_normalize()
        })
        .next()
}

fn aromatic_ring_offsets(
    atom_posits: &[Vec3],
    bond_indices: &[[usize; 2]],
    bond_types: &[BondType],
) -> Vec<Option<Vec3>> {
    let mut offsets = vec![None; bond_indices.len()];
    let mut adjacency = vec![Vec::<(usize, usize)>::new(); atom_posits.len()];

    for (bond_index, [a, b]) in bond_indices.iter().enumerate() {
        if bond_types.get(bond_index) != Some(&BondType::AROMATIC) {
            continue;
        }
        if *a >= atom_posits.len() || *b >= atom_posits.len() {
            continue;
        }
        adjacency[*a].push((*b, bond_index));
        adjacency[*b].push((*a, bond_index));
    }

    for (bond_index, _) in bond_indices.iter().enumerate() {
        if bond_types.get(bond_index) != Some(&BondType::AROMATIC) {
            continue;
        }
        offsets[bond_index] =
            aromatic_bond_inner_offset(bond_index, atom_posits, bond_indices, &adjacency);
    }

    offsets
}

fn aromatic_bond_inner_offset(
    bond_index: usize,
    atom_posits: &[Vec3],
    bond_indices: &[[usize; 2]],
    adjacency: &[Vec<(usize, usize)>],
) -> Option<Vec3> {
    let [start, goal] = *bond_indices.get(bond_index)?;
    if start >= atom_posits.len() || goal >= atom_posits.len() {
        return None;
    }

    let mut queue = VecDeque::from([start]);
    let mut visited = vec![false; atom_posits.len()];
    let mut previous = vec![None::<usize>; atom_posits.len()];
    visited[start] = true;

    while let Some(current) = queue.pop_front() {
        for &(next, edge_index) in &adjacency[current] {
            if edge_index == bond_index || visited[next] {
                continue;
            }

            visited[next] = true;
            previous[next] = Some(current);
            if next == goal {
                break;
            }

            queue.push_back(next);
        }
    }

    if !visited[goal] {
        return None;
    }

    let mut cycle_atoms = vec![goal];
    let mut current = goal;
    while current != start {
        current = previous[current]?;
        cycle_atoms.push(current);
    }

    if cycle_atoms.len() < 3 {
        return None;
    }

    let center = cycle_atoms
        .iter()
        .map(|atom| atom_posits[*atom])
        .sum::<Vec3>()
        / cycle_atoms.len() as f32;

    let bond = atom_posits[goal] - atom_posits[start];
    let dir = bond.try_normalize()?;
    let midpoint = 0.5 * (atom_posits[start] + atom_posits[goal]);
    let inward = center - midpoint;
    let projected = inward - dir * inward.dot(dir);
    projected.try_normalize()
}

impl IntoInstanceGroups for Molecule {
    fn to_instance_group(&self, scale: f32) -> InstanceGroups {
        let mut groups = InstanceGroups {
            spheres: Vec::with_capacity(self.atom_posits.len()),
            sticks: Vec::with_capacity(self.bond_indices.len() * 6),
            outlines: Vec::new(),
        };

        let alpha = self.visual_style.opacity.clamp(0.0, 1.0);
        let material = [Material::default().roughness, Material::default().metallic];
        let aromatic_offsets =
            aromatic_ring_offsets(&self.atom_posits, &self.bond_indices, &self.bond_types);

        if matches!(
            self.style,
            MoleculeStyle::BallAndStick | MoleculeStyle::Sphere
        ) {
            for (i, pos) in self.atom_posits.iter().enumerate() {
                let color = self.get_atom_colors(i);
                groups.spheres.push(SphereInstance::new(
                    [pos[0] * scale, pos[1] * scale, pos[2] * scale],
                    self.atom_types.get(i).map(|x| my_radius(x) * 0.2).unwrap() * scale,
                    [color[0], color[1], color[2], alpha],
                    material,
                ));
            }
        }

        if matches!(
            self.style,
            MoleculeStyle::BallAndStick | MoleculeStyle::Stick
        ) {
            for (i, bond) in self.bond_indices.iter().enumerate() {
                let [a, b] = bond;
                let pos_a = self.atom_posits[*a];
                let pos_b = self.atom_posits[*b];

                let bond_type = self.bond_types.get(i).unwrap_or(&BondType::SINGLE);

                let bond_dir = pos_b - pos_a;
                if bond_dir.length_squared() <= BOND_EPSILON * BOND_EPSILON {
                    continue;
                }
                let Some(dir_n) = bond_dir.try_normalize() else {
                    continue;
                };

                let color_a = self.get_bond_atom_color(*a);
                let color_b = self.get_bond_atom_color(*b);
                let color_a = [color_a[0], color_a[1], color_a[2], alpha];
                let color_b = [color_b[0], color_b[1], color_b[2], alpha];

                let (num_sticks, radius, offset_step) = match (self.style, *bond_type) {
                    (MoleculeStyle::Stick, BondType::SINGLE) => (1, CHIMERAX_STICK_RADIUS, 0.0),
                    (MoleculeStyle::Stick, BondType::DOUBLE) => {
                        let radius = CHIMERAX_STICK_RADIUS * 0.5;
                        (2, radius, radius * 2.0 + CHIMERAX_MULTIBOND_GAP)
                    }
                    (MoleculeStyle::Stick, BondType::TRIPLE) => {
                        let radius = CHIMERAX_STICK_RADIUS / 3.0;
                        (3, radius, radius * 2.0 + CHIMERAX_MULTIBOND_GAP)
                    }
                    (MoleculeStyle::Stick, BondType::AROMATIC) => (1, CHIMERAX_STICK_RADIUS, 0.0),
                    (MoleculeStyle::Stick, _) => (1, CHIMERAX_STICK_RADIUS, 0.0),
                    (_, BondType::SINGLE) => (1, 0.135, 0.22),
                    (_, BondType::DOUBLE) => (2, 0.09, 0.22),
                    (_, BondType::TRIPLE) => (3, 0.05, 0.14),
                    (_, BondType::AROMATIC) => (1, 0.135, 0.0),
                    _ => (1, 0.05, 0.22),
                };
                let off_n = if num_sticks > 1 {
                    bond_offset_direction(*a, *b, &self.atom_posits, &self.bond_indices, dir_n)
                        .or_else(|| {
                            bond_offset_direction(
                                *b,
                                *a,
                                &self.atom_posits,
                                &self.bond_indices,
                                dir_n,
                            )
                        })
                        .unwrap_or_else(|| perpendicular_unit_vector(dir_n))
                } else {
                    Vec3::ZERO
                };

                for k in 0..num_sticks {
                    let offset_mul = (k as f32 - (num_sticks - 1) as f32 * 0.5) * offset_step;

                    let pos_a_k = pos_a + off_n * offset_mul;
                    let pos_b_k = pos_b + off_n * offset_mul;
                    let midpoint = 0.5 * (pos_a_k + pos_b_k);
                    let scaled_pos_a = scaled_point(pos_a_k.to_array(), scale);
                    let scaled_pos_b = scaled_point(pos_b_k.to_array(), scale);
                    let scaled_midpoint = scaled_point(midpoint.to_array(), scale);
                    let scaled_radius = radius * scale;

                    if matches!(self.style, MoleculeStyle::Stick) {
                        groups.spheres.push(SphereInstance::new(
                            scaled_pos_a,
                            scaled_radius,
                            color_a,
                            material,
                        ));
                        groups.spheres.push(SphereInstance::new(
                            scaled_pos_b,
                            scaled_radius,
                            color_b,
                            material,
                        ));
                    }

                    groups.sticks.push(StickInstance::new(
                        scaled_pos_a,
                        scaled_midpoint,
                        scaled_radius,
                        color_a,
                        material,
                    ));

                    groups.sticks.push(StickInstance::new(
                        scaled_pos_b,
                        scaled_midpoint,
                        scaled_radius,
                        color_b,
                        material,
                    ));
                }

                if *bond_type == BondType::AROMATIC {
                    let aromatic_radius = radius * AROMATIC_LINE_RADIUS_SCALE;
                    let aromatic_offset = radius + aromatic_radius + AROMATIC_LINE_GAP;
                    if let Some(inward) = aromatic_offsets[i] {
                        let pos_a_k = pos_a + inward * aromatic_offset;
                        let pos_b_k = pos_b + inward * aromatic_offset;
                        let midpoint = 0.5 * (pos_a_k + pos_b_k);
                        let scaled_pos_a = scaled_point(pos_a_k.to_array(), scale);
                        let scaled_pos_b = scaled_point(pos_b_k.to_array(), scale);
                        let scaled_midpoint = scaled_point(midpoint.to_array(), scale);
                        let scaled_radius = aromatic_radius * scale;

                        if matches!(self.style, MoleculeStyle::Stick) {
                            groups.spheres.push(SphereInstance::new(
                                scaled_pos_a,
                                scaled_radius,
                                color_a,
                                material,
                            ));
                            groups.spheres.push(SphereInstance::new(
                                scaled_pos_b,
                                scaled_radius,
                                color_b,
                                material,
                            ));
                        }

                        groups.sticks.push(StickInstance::new(
                            scaled_pos_a,
                            scaled_midpoint,
                            scaled_radius,
                            color_a,
                            material,
                        ));
                        groups.sticks.push(StickInstance::new(
                            scaled_pos_b,
                            scaled_midpoint,
                            scaled_radius,
                            color_b,
                            material,
                        ));
                    }
                }
            }
        }
        if self.outline.enabled && self.outline.width > 0.0 {
            let mut settings = self.outline;
            settings.width *= scale;
            groups.outlines.push(OutlineInstanceGroup {
                settings,
                spheres: groups.spheres.clone(),
                sticks: groups.sticks.clone(),
            });
        }

        groups
    }
}

impl Stylable for Molecule {
    fn style_mut(&mut self) -> &mut Material {
        &mut self.visual_style
    }
}

crate::impl_stylable_methods!(Molecule, visual_style);

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
    use cosmolkit::io::sdf::read_sdf_from_str_with_coordinate_mode;

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
    fn from_sdf_preserves_explicit_hydrogens() {
        let sdf = "\
methane
  cosmol_viewer

  5  4  0  0  0  0  0  0  0  0999 V2000
    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    0.6291    0.6291    0.6291 H   0  0  0  0  0  0  0  0  0  0  0  0
   -0.6291   -0.6291    0.6291 H   0  0  0  0  0  0  0  0  0  0  0  0
   -0.6291    0.6291   -0.6291 H   0  0  0  0  0  0  0  0  0  0  0  0
    0.6291   -0.6291   -0.6291 H   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0  0  0  0
  1  3  1  0  0  0  0
  1  4  1  0  0  0  0
  1  5  1  0  0  0  0
M  END
$$$$
";

        let molecule = Molecule::from_sdf(sdf).expect("methane SDF should parse");

        assert_eq!(molecule.atom_types.len(), 5);
        assert_eq!(
            molecule
                .atom_types
                .iter()
                .filter(|element| **element == Element::H)
                .count(),
            4
        );
        assert_eq!(molecule.bond_indices.len(), 4);
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
    fn zero_length_neighbor_bond_does_not_hide_valid_single_bond() {
        let molecule = Molecule {
            style: MoleculeStyle::BallAndStick,
            atom_types: vec![Element::O, Element::C, Element::H],
            atom_colors: None,
            atom_posits: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
            ],
            bond_types: vec![BondType::SINGLE, BondType::SINGLE],
            bond_indices: vec![[0, 1], [0, 2]],
            quality: 6,
            visual_style: Material {
                opacity: 1.0,
                visible: true,
                ..Default::default()
            },
            interaction: Default::default(),
            outline: OutlineSettings::default(),
        };

        let groups = molecule.to_instance_group(1.0);

        assert_eq!(groups.sticks.len(), 2);
        assert!(groups.sticks.iter().all(|stick| {
            stick
                .start
                .iter()
                .chain(stick.end.iter())
                .all(|value| value.is_finite())
                && stick.radius.is_finite()
        }));
    }

    #[test]
    fn molecule_style_controls_generated_instances() {
        let molecule = Molecule::from_atom_bond_data(
            vec![6, 8],
            vec![[0.0, 0.0, 0.0], [1.2, 0.0, 0.0]],
            vec![1],
            vec![[0, 1]],
            None,
        )
        .expect("minimal molecule should convert");

        let ball_and_stick = molecule.clone().ball_and_stick().to_instance_group(1.0);
        assert_eq!(ball_and_stick.spheres.len(), 2);
        assert_eq!(ball_and_stick.sticks.len(), 2);

        let stick = molecule.clone().stick().to_instance_group(1.0);
        assert_eq!(stick.spheres.len(), 2);
        assert_eq!(stick.sticks.len(), 2);
        assert!(
            stick
                .spheres
                .iter()
                .all(|sphere| (sphere.radius - CHIMERAX_STICK_RADIUS).abs() < 1.0e-6)
        );

        let sphere = molecule.sphere().to_instance_group(1.0);
        assert_eq!(sphere.spheres.len(), 2);
        assert!(sphere.sticks.is_empty());
    }

    #[test]
    fn stick_style_preserves_bond_order_with_chimerax_scaled_spacing() {
        let molecule = Molecule::from_atom_bond_data(
            vec![6, 8, 7],
            vec![[0.0, 0.0, 0.0], [1.2, 0.0, 0.0], [2.4, 0.0, 0.0]],
            vec![2, 3],
            vec![[0, 1], [1, 2]],
            None,
        )
        .expect("minimal molecule should convert");

        let groups = molecule.stick().to_instance_group(1.0);

        assert_eq!(groups.sticks.len(), 10);
        assert_eq!(groups.spheres.len(), 10);

        let double_radius = CHIMERAX_STICK_RADIUS * 0.5;
        let triple_radius = CHIMERAX_STICK_RADIUS / 3.0;
        let double_spacing = double_radius * 2.0 + CHIMERAX_MULTIBOND_GAP;
        let triple_spacing = triple_radius * 2.0 + CHIMERAX_MULTIBOND_GAP;

        assert!(
            groups.spheres[0..4]
                .iter()
                .all(|sphere| (sphere.radius - double_radius).abs() < 1.0e-6)
        );
        assert!(
            (distance_between(groups.spheres[0].position, groups.spheres[2].position)
                - double_spacing)
                .abs()
                < 1.0e-6
        );
        assert!(
            groups.spheres[4..10]
                .iter()
                .all(|sphere| (sphere.radius - triple_radius).abs() < 1.0e-6)
        );
        assert!(
            (distance_between(groups.spheres[4].position, groups.spheres[6].position)
                - triple_spacing)
                .abs()
                < 1.0e-6
        );
    }

    #[test]
    fn aromatic_ring_renders_single_sticks_with_inner_aromatic_lines() {
        let angle = std::f32::consts::TAU / 6.0;
        let atom_posits = (0..6)
            .map(|i| {
                let theta = angle * i as f32;
                [theta.cos(), theta.sin(), 0.0]
            })
            .collect::<Vec<_>>();
        let bonds = (0..6).map(|i| [i, (i + 1) % 6]).collect::<Vec<_>>();
        let molecule =
            Molecule::from_atom_bond_data(vec![6; 6], atom_posits, vec![5; 6], bonds, None)
                .expect("aromatic ring should convert");

        let ball_and_stick = molecule.clone().ball_and_stick().to_instance_group(1.0);
        assert_eq!(ball_and_stick.spheres.len(), 6);
        assert_eq!(ball_and_stick.sticks.len(), 24);
        assert_eq!(
            ball_and_stick
                .sticks
                .iter()
                .filter(|stick| (stick.radius - 0.135).abs() < 1.0e-6)
                .count(),
            12
        );
        assert_eq!(
            ball_and_stick
                .sticks
                .iter()
                .filter(|stick| {
                    (stick.radius - 0.135 * AROMATIC_LINE_RADIUS_SCALE).abs() < 1.0e-6
                })
                .count(),
            12
        );

        let stick = molecule.stick().to_instance_group(1.0);
        let aromatic_radius = CHIMERAX_STICK_RADIUS * AROMATIC_LINE_RADIUS_SCALE;
        assert_eq!(stick.sticks.len(), 24);
        assert_eq!(stick.spheres.len(), 24);
        assert_eq!(
            stick
                .spheres
                .iter()
                .filter(|sphere| (sphere.radius - CHIMERAX_STICK_RADIUS).abs() < 1.0e-6)
                .count(),
            12
        );
        assert_eq!(
            stick
                .spheres
                .iter()
                .filter(|sphere| (sphere.radius - aromatic_radius).abs() < 1.0e-6)
                .count(),
            12
        );
    }

    #[test]
    fn non_ring_aromatic_bond_renders_as_single_stick_only() {
        let molecule = Molecule::from_atom_bond_data(
            vec![6, 6],
            vec![[0.0, 0.0, 0.0], [1.4, 0.0, 0.0]],
            vec![5],
            vec![[0, 1]],
            None,
        )
        .expect("linear aromatic bond should convert");

        let ball_and_stick = molecule.clone().ball_and_stick().to_instance_group(1.0);
        assert_eq!(ball_and_stick.spheres.len(), 2);
        assert_eq!(ball_and_stick.sticks.len(), 2);
        assert!(
            ball_and_stick
                .sticks
                .iter()
                .all(|stick| (stick.radius - 0.135).abs() < 1.0e-6)
        );

        let stick = molecule.stick().to_instance_group(1.0);
        assert_eq!(stick.spheres.len(), 2);
        assert_eq!(stick.sticks.len(), 2);
        assert!(
            stick
                .spheres
                .iter()
                .all(|sphere| (sphere.radius - CHIMERAX_STICK_RADIUS).abs() < 1.0e-6)
        );
    }

    fn distance_between(a: [f32; 3], b: [f32; 3]) -> f32 {
        Vec3::from(a).distance(Vec3::from(b))
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

        assert_eq!(molecule.atom_types, vec![Element::DUMMY, Element::DUMMY]);
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
            vec![Element::C, Element::O, Element::DUMMY, Element::DUMMY]
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
