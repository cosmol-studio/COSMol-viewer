//! Display configuration, independent of scientific objects and selections.

use crate::utils::{Color, Material, OutlineSettings};
use cosmolkit::Element;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

/// Reusable element colors and optional overrides by zero-based CK atom ID.
/// Unspecified elements use the built-in molecular colors.
#[derive(Debug, Clone, Default)]
pub struct MoleculeColors {
    elements: Option<Arc<HashMap<Element, Vec3>>>,
    atoms: Option<Arc<HashMap<usize, Vec3>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum MoleculeColorError {
    #[error("unknown element symbol {0:?}")]
    InvalidElement(String),
    #[error("RGB components must be finite and between 0 and 1")]
    InvalidColor,
    #[error("invalid atom ID {0:?}; expected a nonnegative integer")]
    InvalidAtomId(String),
}

impl MoleculeColors {
    pub fn new(
        elements: impl IntoIterator<Item = (impl AsRef<str>, Color)>,
    ) -> Result<Self, MoleculeColorError> {
        let elements = elements
            .into_iter()
            .map(|(symbol, color)| {
                let symbol = symbol.as_ref();
                let element = Element::from_symbol(symbol)
                    .ok_or_else(|| MoleculeColorError::InvalidElement(symbol.to_owned()))?;
                Ok((element, checked_color(color)?))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Self {
            elements: (!elements.is_empty()).then(|| Arc::new(elements)),
            atoms: None,
        })
    }

    /// Return a new scheme with the given atom colors, sharing the element colors.
    pub fn with_atom_colors(
        &self,
        atoms: impl IntoIterator<Item = (usize, Color)>,
    ) -> Result<Self, MoleculeColorError> {
        let atoms = atoms
            .into_iter()
            .map(|(id, color)| Ok((id, checked_color(color)?)))
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Self {
            elements: self.elements.clone(),
            atoms: (!atoms.is_empty()).then(|| Arc::new(atoms)),
        })
    }

    pub(crate) fn element(&self, element: &Element) -> Option<Vec3> {
        self.elements.as_ref()?.get(element).copied()
    }

    pub(crate) fn atom(&self, id: usize) -> Option<Vec3> {
        self.atoms.as_ref()?.get(&id).copied()
    }

    pub(crate) fn atom_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.atoms.iter().flat_map(|atoms| atoms.keys().copied())
    }
}

fn checked_color(color: Color) -> Result<Vec3, MoleculeColorError> {
    if color.0.to_array().iter().all(|x| (0.0..=1.0).contains(x)) {
        Ok(color.0)
    } else {
        Err(MoleculeColorError::InvalidColor)
    }
}

/// A display style paired with an optional reusable color scheme.
#[derive(Debug, Clone)]
pub struct DisplayStyle {
    #[doc(hidden)]
    pub representation: Representation,
    #[doc(hidden)]
    pub molecule_colors: Option<MoleculeColors>,
}

impl<S: Into<Representation>> From<S> for DisplayStyle {
    fn from(style: S) -> Self {
        Self {
            representation: style.into(),
            molecule_colors: None,
        }
    }
}

impl DisplayStyle {
    /// Apply a scheme to a molecular glyph style.
    pub fn set_molecule_colors(&mut self, colors: &MoleculeColors) -> Result<(), &'static str> {
        self.representation.molecular_options_mut()?;
        self.molecule_colors = Some(colors.clone());
        Ok(())
    }
}

/// Which stored molecular coordinates to display. Reads never generate coordinates.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum Coordinates {
    /// Use the unique stored 3D conformer, otherwise the first stored 2D layout.
    /// Multiple 3D conformers require an explicit ID.
    #[default]
    Auto,
    TwoD,
    ThreeD {
        id: usize,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct MolecularOptions {
    pub material: Material,
    pub outline: OutlineSettings,
    pub coordinates: Coordinates,
    pub radius_scale: f32,
}

impl Default for MolecularOptions {
    fn default() -> Self {
        Self {
            material: Material::default(),
            outline: OutlineSettings::default(),
            coordinates: Coordinates::Auto,
            radius_scale: 1.0,
        }
    }
}

macro_rules! molecular_style {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
        pub struct $name {
            pub(crate) options: MolecularOptions,
        }

        impl $name {
            pub fn color<C: Into<Color>>(mut self, color: C) -> Self {
                self.options.material.color = Some(color.into().into());
                self
            }
            /// Apply a reusable color scheme to this display.
            pub fn colors(self, colors: &MoleculeColors) -> DisplayStyle {
                DisplayStyle {
                    representation: self.into(),
                    molecule_colors: Some(colors.clone()),
                }
            }
            pub fn opacity(mut self, opacity: f32) -> Self {
                assert!(opacity.is_finite() && (0.0..=1.0).contains(&opacity));
                self.options.material.opacity = opacity;
                self
            }
            pub fn roughness(mut self, roughness: f32) -> Self {
                assert!(roughness.is_finite() && (0.0..=1.0).contains(&roughness));
                self.options.material.roughness = roughness;
                self
            }
            pub fn metallic(mut self, metallic: f32) -> Self {
                assert!(metallic.is_finite() && (0.0..=1.0).contains(&metallic));
                self.options.material.metallic = metallic;
                self
            }
            /// Multiply glyph radii, not the source's coordinates.
            pub fn scale(mut self, scale: f32) -> Self {
                assert!(scale.is_finite() && scale > 0.0);
                self.options.radius_scale = scale;
                self
            }
            pub fn outline<C: Into<Color>>(mut self, color: C, width: f32) -> Self {
                assert!(width.is_finite() && width >= 0.0);
                self.options.outline = OutlineSettings {
                    enabled: width > 0.0,
                    color: color.into().into(),
                    width,
                };
                self
            }
            pub fn coordinates(mut self, coordinates: Coordinates) -> Self {
                self.options.coordinates = coordinates;
                self
            }
        }
    };
}

molecular_style!(
    BallAndStick,
    "Element-colored atom balls and bond-order sticks."
);
molecular_style!(
    Sphere,
    "Van der Waals atom spheres; not a positioned primitive sphere."
);
molecular_style!(
    Stick,
    "Bond-order sticks with rounded ends; not a primitive stick."
);

/// Protein backbone ribbons. Scientific residue identities stay in COSMolKit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cartoon {
    pub(crate) material: Material,
    pub(crate) rainbow: bool,
}

impl Default for Cartoon {
    fn default() -> Self {
        Self {
            material: Material {
                roughness: 0.7,
                ..Default::default()
            },
            rainbow: false,
        }
    }
}

impl Cartoon {
    pub fn rainbow_residues(mut self) -> Self {
        self.rainbow = true;
        self.material.color = None;
        self
    }
}
crate::impl_stylable_methods!(Cartoon, material);

/// Solvent-excluded (or explicitly solvent-accessible) protein surface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Surface {
    pub(crate) material: Material,
    pub probe_radius: f32,
    pub grid_spacing: f32,
    pub solvent_accessible: bool,
    pub sharp_boundaries: bool,
}

impl Default for Surface {
    fn default() -> Self {
        Self {
            material: Material {
                roughness: 0.7,
                ..Default::default()
            },
            probe_radius: 1.4,
            grid_spacing: 0.5,
            solvent_accessible: false,
            sharp_boundaries: true,
        }
    }
}
crate::impl_stylable_methods!(Surface, material);

/// A display style. Incompatible selection/style combinations fail explicitly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Representation {
    BallAndStick(BallAndStick),
    Sphere(Sphere),
    Stick(Stick),
    Cartoon(Cartoon),
    Surface(Surface),
}

impl Representation {
    fn material_mut(&mut self) -> &mut Material {
        match self {
            Self::BallAndStick(s) => &mut s.options.material,
            Self::Sphere(s) => &mut s.options.material,
            Self::Stick(s) => &mut s.options.material,
            Self::Cartoon(s) => &mut s.material,
            Self::Surface(s) => &mut s.material,
        }
    }
    pub fn set_color(&mut self, color: impl Into<Color>) {
        self.material_mut().color = Some(color.into().into());
    }
    pub fn set_opacity(&mut self, value: f32) -> Result<(), &'static str> {
        Self::unit_interval(value)?;
        self.material_mut().opacity = value;
        Ok(())
    }
    pub fn set_roughness(&mut self, value: f32) -> Result<(), &'static str> {
        Self::unit_interval(value)?;
        self.material_mut().roughness = value;
        Ok(())
    }
    pub fn set_metallic(&mut self, value: f32) -> Result<(), &'static str> {
        Self::unit_interval(value)?;
        self.material_mut().metallic = value;
        Ok(())
    }
    fn unit_interval(value: f32) -> Result<(), &'static str> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(())
        } else {
            Err("value must be finite and between 0 and 1")
        }
    }
    fn molecular_options_mut(&mut self) -> Result<&mut MolecularOptions, &'static str> {
        match self {
            Self::BallAndStick(s) => Ok(&mut s.options),
            Self::Sphere(s) => Ok(&mut s.options),
            Self::Stick(s) => Ok(&mut s.options),
            _ => Err("this option requires a molecular glyph style"),
        }
    }
    pub fn set_radius_scale(&mut self, value: f32) -> Result<(), &'static str> {
        if !value.is_finite() || value <= 0.0 {
            return Err("scale must be finite and positive");
        }
        self.molecular_options_mut()?.radius_scale = value;
        Ok(())
    }
    pub fn set_coordinates(&mut self, value: Coordinates) -> Result<(), &'static str> {
        self.molecular_options_mut()?.coordinates = value;
        Ok(())
    }
    pub fn set_outline(&mut self, color: impl Into<Color>, width: f32) -> Result<(), &'static str> {
        if !width.is_finite() || width < 0.0 {
            return Err("outline width must be finite and nonnegative");
        }
        self.molecular_options_mut()?.outline = OutlineSettings {
            enabled: width > 0.0,
            color: color.into().into(),
            width,
        };
        Ok(())
    }
    pub fn set_rainbow_residues(&mut self) -> Result<(), &'static str> {
        match self {
            Self::Cartoon(s) => {
                *s = s.rainbow_residues();
                Ok(())
            }
            _ => Err("rainbow residues requires Cartoon"),
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::BallAndStick(_) => "BallAndStick",
            Self::Sphere(_) => "Sphere",
            Self::Stick(_) => "Stick",
            Self::Cartoon(_) => "Cartoon",
            Self::Surface(_) => "Surface",
        }
    }
}

macro_rules! representation {
    ($($name:ident),* $(,)?) => {$(
        impl From<$name> for Representation {
            fn from(style: $name) -> Self { Self::$name(style) }
        }
    )*};
}
representation!(BallAndStick, Sphere, Stick, Cartoon, Surface);
