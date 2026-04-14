use glam::Vec3;
use na_seq::AaIdent;
use na_seq::{AminoAcid, AtomTypeInRes, Element};
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io;
use std::io::ErrorKind;
use std::str::FromStr;

#[derive(Clone, Debug, Default)]
pub struct AtomGeneric {
    /// A unique identifier for this atom, within its molecule. This may originate from data in
    /// mmCIF files, Mol2, SDF files, etc.
    pub serial_number: u32,
    pub posit: Vec3,
    pub element: Element,
    /// This identifier will be unique within a given residue. For example, within an
    /// amino acid on a protein. Different residues will have different sets of these.
    /// e.g. "CG1", "CA", "O", "C", "HA", "CD", "C9" etc.
    /// todo: This setup might be protein/aa specific.
    pub type_in_res: Option<AtomTypeInRes>,
    /// There are too many variants of this (with different numbers) for lipids, nucleic
    /// acids etc to use an enum effectively.
    pub type_in_res_general: Option<String>,
    /// Used by Amber and other force fields to apply the correct molecular dynamics parameters for
    /// this atom.
    /// E.g. "c6", "ca", "n3", "ha", "h0" etc, as seen in Mol2 files from AMBER.
    /// e.g.: "ha": hydrogen attached to an aromatic carbon.
    /// "ho": hydrogen on a hydroxyl oxygen
    /// "n3": sp³ nitrogen with three substitutes
    /// "c6": sp² carbon in a pure six-membered aromatic ring (new in GAFF2; lets GAFF distinguish
    /// a benzene carbon from other aromatic caca carbons)
    /// For proteins, this appears to be the same as for `name`.
    pub force_field_type: Option<String>,
    /// An atom-centered electric charge, used in molecular dynamics simulations. In elementary charge units.
    /// These are sometimes loaded from Amber-provided Mol2 or SDF files, and sometimes added after.
    /// We get partial charge for ligands from (e.g. Amber-provided) Mol files, so we load it from the atom, vice
    /// the loaded FF params. Convert to appropriate units prior to running dynamics.
    pub partial_charge: Option<f32>,
    /// Indicates, in proteins, that the atom isn't part of an amino acid. E.g., water or
    /// ligands.
    pub hetero: bool,
    pub occupancy: Option<f32>,
    /// Used by mmCIF files to store alternate conformations. If this isn't None, there may
    /// be, for example, an "A" and "B" variant of this atom at slightly different positions.
    pub alt_conformation_id: Option<String>,
}

impl Display for AtomGeneric {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let ff_type = match &self.force_field_type {
            Some(f) => f,
            None => "None",
        };

        let q = match &self.partial_charge {
            Some(q_) => format!("{q_:.3}"),
            None => "None".to_string(),
        };

        write!(
            f,
            "Atom {}: {}, {}. {:?}, ff: {ff_type}, q: {q}",
            self.serial_number,
            self.element.to_letter(),
            self.posit,
            self.type_in_res,
        )?;

        if self.hetero {
            write!(f, ", Het")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChainGeneric {
    pub id: String,
    // todo: Do we want both residues and atoms stored here? It's an overconstraint.
    /// Serial number
    pub residue_sns: Vec<u32>,
    /// Serial number
    pub atom_sns: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResidueType {
    #[serde(with = "aa_serde")]
    AminoAcid(AminoAcid),
    Water,
    Other(String),
}

impl From<&ResidueType> for ResidueType {
    fn from(res_type: &ResidueType) -> Self {
        match res_type {
            ResidueType::AminoAcid(a) => ResidueType::AminoAcid(*a),
            ResidueType::Water => ResidueType::Water,
            ResidueType::Other(s) => ResidueType::Other(s.clone()),
        }
    }
}

impl Display for ResidueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match &self {
            ResidueType::Other(n) => n.clone(),
            ResidueType::Water => "Water".to_string(),
            ResidueType::AminoAcid(aa) => aa.to_string(),
        };

        write!(f, "{name}")
    }
}

impl Default for ResidueType {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl ResidueType {
    /// Parses from the "name" field in common text-based formats lik CIF, PDB, and PDBQT.
    pub fn from_str(name: &str) -> Self {
        if name.to_uppercase() == "HOH" {
            ResidueType::Water
        } else {
            match AminoAcid::from_str(name) {
                Ok(aa) => ResidueType::AminoAcid(aa),
                Err(_) => ResidueType::Other(name.to_owned()),
            }
        }
    }
}

mod aa_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(aa: &AminoAcid, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&aa.to_str(AaIdent::OneLetter))
    }

    pub fn deserialize<'de, D>(d: D) -> Result<AminoAcid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(d)?;
        AminoAcid::from_str(&name)
            .map_err(|_| serde::de::Error::custom(format!("Invalid amino acid string: {}", name)))
    }
}

#[derive(Debug, Clone)]
pub struct ResidueGeneric {
    /// We use serial number of display, search etc, and array index to select. Residue serial number is not
    /// unique in the molecule; only in the chain.
    pub serial_number: u32,
    pub res_type: ResidueType,
    /// Serial number
    pub atom_sns: Vec<u32>,
    pub end: ResidueEnd,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ResidueEnd {
    Internal,
    NTerminus,
    CTerminus,
    /// Not part of a protein/polypeptide.
    Hetero,
}

/// These are the Mol2 standard types, unless otherwise noted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BondType {
    Single,
    Double,
    Triple,
    Aromatic,
    Amide,
    Dummy,
    Unknown,
    NotConnected,
    /// mmCIF, rare
    Quadruple,
    /// mmCIF. Distinct from aromatic; doesn't need to be a classic ring.
    Delocalized,
    /// mmCif; mostly for macromolecular components
    PolymericLink,
}

impl Display for BondType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Single => "Single",
            Self::Double => "Double",
            Self::Triple => "Triple",
            Self::Aromatic => "Aromatic",
            Self::Amide => "Amide",
            Self::Dummy => "Dummy",
            Self::Unknown => "Unknown",
            Self::NotConnected => "Not connected",
            Self::Quadruple => "Quadruple",
            Self::Delocalized => "Delocalized",
            Self::PolymericLink => "Polymeric link",
        };
        write!(f, "{name}")
    }
}

impl BondType {
    pub fn order(self) -> f32 {
        match self {
            Self::Aromatic => 1.5,
            Self::Double => 2.,
            Self::Triple => 3.,
            Self::Quadruple => 4.,
            _ => 1.,
        }
    }

    /// A shorthand, visual string.
    pub fn to_visual_str(&self) -> String {
        match self {
            Self::Single => "-",
            Self::Double => "=",
            Self::Triple => "≡",
            Self::Aromatic => "=–",
            Self::Amide => "-am-",
            Self::Dummy => "-",
            Self::Unknown => "-un-",
            Self::NotConnected => "-nc-",
            Self::Quadruple => "-#-",
            Self::Delocalized => "-delo-",
            Self::PolymericLink => "-poly-",
        }
        .to_string()
    }

    /// Return the exact MOL2 bond-type token as an owned `String`.
    /// (Use `&'static str` if you never need it allocated.)
    pub fn to_mol2_str(&self) -> String {
        match self {
            Self::Single => "1",
            Self::Double => "2",
            Self::Triple => "3",
            Self::Aromatic => "ar",
            Self::Amide => "am",
            Self::Dummy => "du",
            Self::Unknown => "un",
            Self::NotConnected => "nc",
            Self::Quadruple => "quad",
            Self::Delocalized => "delo",
            Self::PolymericLink => "poly",
        }
        .to_string()
    }

    /// SDF format uses a truncated set, and does things like mark every other
    /// aromatic bond as double.
    pub fn to_str_sdf(&self) -> String {
        match self {
            Self::Single | Self::Double | Self::Triple => *self,
            Self::Aromatic => return "4".to_string(),
            _ => Self::Single,
        }
        .to_mol2_str()
    }
}

impl FromStr for BondType {
    type Err = io::Error;

    /// Can ingest from mol2, SDF, and mmCIF formats.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "1" | "sing" => Ok(BondType::Single),
            "2" | "doub" => Ok(BondType::Double),
            "3" | "trip" => Ok(BondType::Triple),
            "4" | "ar" | "arom" => Ok(BondType::Aromatic),
            "am" => Ok(BondType::Amide),
            "du" => Ok(BondType::Dummy),
            "un" => Ok(BondType::Unknown),
            "nc" => Ok(BondType::NotConnected),
            "quad" => Ok(BondType::Quadruple),
            "delo" => Ok(BondType::Delocalized),
            "poly" => Ok(BondType::PolymericLink),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Invalid BondType: {s}"),
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BondGeneric {
    pub bond_type: BondType,
    /// You may wish to augment these serial numbers with atom indices in downstream
    /// applications, for lookup speed.
    pub atom_0_sn: u32,
    pub atom_1_sn: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Residue {
    #[serde(with = "aa_serde")]
    pub residue_type: AminoAcid, // e.g. "ALA", "GLY"
    // pub residue_type: ResidueType, // e.g. "ALA", "GLY"
    pub sns: usize, // PDB numbering or sequential

    // Minimum for cartoon backbone
    pub c: Vec3,         // or pseudo-CB for glycine
    pub n: Vec3,         // C-alpha coordinates
    pub ca: Vec3,        // C-alpha coordinates
    pub o: Vec3,         // C-alpha coordinates
    pub h: Option<Vec3>, // C-alpha coordinates

    // Secondary structure tag
    pub ss: Option<SecondaryStructure>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondaryStructure {
    Helix,
    Sheet,
    Coil,
    Turn,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RibbonResidueInfo {
    pub ss: SecondaryStructure,
    pub helix_id: Option<usize>,
    pub sheet_id: Option<usize>,
}

impl Default for RibbonResidueInfo {
    fn default() -> Self {
        Self {
            ss: SecondaryStructure::Coil,
            helix_id: None,
            sheet_id: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PharmacophoreType {
    Acceptor,
    Donor,
    Cation,
    Rings,
}

#[derive(Clone, Debug)]
pub struct PharmacaphoreFeatures {
    pub atom_sns: Vec<u32>,       // 1-based atom indices (SDF serial numbers)
    pub type_: PharmacophoreType, // e.g. "acceptor", "cation", "rings"
}
