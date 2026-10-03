use cosmolkit::ResidueCode;
use glam::Vec3;
use serde::{Deserialize, Serialize};

// Temporary adapter until CK provides Serde for ResidueCode. Preserve CK's
// residue names rather than collapsing modified residues to one-letter codes.
mod residue_code_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(code: &ResidueCode, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let name = if *code == ResidueCode::UNKNOWN {
            "UNKNOWN"
        } else {
            cosmolkit::residue_info(usize::from(code.as_u16())).name
        };
        s.serialize_str(name)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<ResidueCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(d)?;
        if name == "UNKNOWN" {
            return Ok(ResidueCode::UNKNOWN);
        }
        let info = cosmolkit::find_residue_info(&name);
        if !info.found() {
            return Err(serde::de::Error::custom(format!(
                "Invalid CK residue name: {name}"
            )));
        }
        Ok(info.code)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Residue {
    /// CK residue identity, temporarily serialized using the CK residue name.
    #[serde(with = "residue_code_serde")]
    pub residue_type: ResidueCode,
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
