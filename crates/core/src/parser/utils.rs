use glam::Vec3;
use na_seq::{AaIdent, AminoAcid};
use serde::{Deserialize, Serialize};

mod aa_serde {
    use super::*;
    use serde::{Deserializer, Serializer};
    use std::str::FromStr;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Residue {
    #[serde(with = "aa_serde")]
    pub residue_type: AminoAcid,
    pub sns: usize,

    pub c: Vec3,
    pub n: Vec3,
    pub ca: Vec3,
    pub o: Vec3,
    pub h: Option<Vec3>,

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
