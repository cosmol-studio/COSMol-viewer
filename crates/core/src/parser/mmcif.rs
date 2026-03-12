use crate::parser::dssp::SecondaryStructureCalculator;
use crate::parser::utils::ResidueType;
use crate::parser::utils::{
    AtomGeneric, ChainGeneric, Residue, ResidueEnd, ResidueGeneric, SecondaryStructure,
};
pub use crate::utils::{Logger, RustLogger};
use glam::Vec3;
use na_seq::{AtomTypeInRes, Element};
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io;
use std::io::ErrorKind;
use std::str::FromStr;

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct MmCif {
    pub ident: String,
    pub metadata: HashMap<String, String>,
    pub atoms: Vec<AtomGeneric>,
    // This is sometimes included in mmCIF files, although seems to be absent
    // from most (all?) on RCSB PDB.
    // pub bonds: Vec<BondGeneric>,
    pub chains: Vec<ChainGeneric>,
    pub residues: Vec<ResidueGeneric>,
    pub secondary_structure: Vec<BackboneSS>,
    pub experimental_method: Option<ExperimentalMethod>,
}

impl MmCif {
    pub fn new(text: &str) -> io::Result<Self> {
        // todo: For these `new` methods in general that take a &str param: Should we use
        // todo R: Reed + Seek instead, and pass a Cursor or File object? Probably doesn't matter.
        // todo Either way, we should keep it consistent between the files.

        // todo: This is far too slow.

        let mut metadata = HashMap::<String, String>::new();
        let mut atoms = Vec::<AtomGeneric>::new();
        let mut residues = Vec::<ResidueGeneric>::new();
        let mut chains = Vec::<ChainGeneric>::new();
        let mut res_idx = HashMap::<(String, u32), usize>::new();
        let mut chain_idx = HashMap::<String, usize>::new();

        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        let n = lines.len();

        let mut experimental_method: Option<ExperimentalMethod> = None;

        let method_re = Regex::new(r#"^_exptl\.method\s+['"]([^'"]+)['"]\s*$"#).unwrap();

        while i < n {
            let mut line = lines[i].trim();
            if line.is_empty() {
                i += 1;
                continue;
            }

            if let Some(caps) = method_re.captures(line)
                && let Ok(m) = caps[1].to_string().parse()
            {
                experimental_method = Some(m);
            }

            if line == "loop_" {
                i += 1;
                let mut headers = Vec::<&str>::new();
                while i < n {
                    line = lines[i].trim();
                    if line.starts_with('_') {
                        headers.push(line);
                        i += 1;
                    } else {
                        break;
                    }
                }

                // If not an atom loops, skip first rows.
                if !headers
                    .first()
                    .is_some_and(|h| h.starts_with("_atom_site."))
                {
                    while i < n {
                        line = lines[i].trim();
                        if line == "#" || line == "loop_" || line.starts_with('_') {
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }

                let col = |tag: &str| -> io::Result<usize> {
                    headers.iter().position(|h| *h == tag).ok_or_else(|| {
                        io::Error::new(ErrorKind::InvalidData, format!("mmCIF missing {tag}"))
                    })
                };
                let het = col("_atom_site.group_PDB")?;
                let c_id = col("_atom_site.id")?;
                let c_x = col("_atom_site.Cartn_x")?;
                let c_y = col("_atom_site.Cartn_y")?;
                let c_z = col("_atom_site.Cartn_z")?;
                let c_el = col("_atom_site.type_symbol")?;
                let c_name = col("_atom_site.label_atom_id")?;
                let c_alt_id = col("_atom_site.label_alt_id")?;
                let c_res = col("_atom_site.label_comp_id")?;
                let c_chain = col("_atom_site.label_asym_id")?;
                let c_res_sn = col("_atom_site.label_seq_id")?;
                let c_occ = col("_atom_site.occupancy")?;

                while i < n {
                    line = lines[i].trim();
                    if line.is_empty() || line == "#" || line == "loop_" || line.starts_with('_') {
                        break;
                    }
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.len() < headers.len() {
                        i += 1;
                        continue;
                    }

                    // Atom lines.
                    let hetero = fields[het].trim() == "HETATM";

                    let serial_number = fields[c_id].parse::<u32>().unwrap_or(0);
                    let x = fields[c_x].parse::<f64>().unwrap_or(0.0);
                    let y = fields[c_y].parse::<f64>().unwrap_or(0.0);
                    let z = fields[c_z].parse::<f64>().unwrap_or(0.0);

                    let element = Element::from_letter(fields[c_el])?;
                    let atom_name = fields[c_name];

                    let alt_conformation_id = if fields[c_alt_id] == "." {
                        None
                    } else {
                        Some(fields[c_alt_id].to_string())
                    };

                    let type_in_res = if hetero {
                        if !atom_name.is_empty() {
                            Some(AtomTypeInRes::Hetero(atom_name.to_string()))
                        } else {
                            None
                        }
                    } else {
                        AtomTypeInRes::from_str(atom_name).ok()
                    };

                    let occ = match fields[c_occ] {
                        "?" | "." => None,
                        v => v.parse().ok(),
                    };

                    atoms.push(AtomGeneric {
                        serial_number,
                        posit: Vec3::new(x as f32, y as f32, z as f32),
                        element,
                        type_in_res,
                        occupancy: occ,
                        hetero,
                        alt_conformation_id,
                        ..Default::default()
                    });

                    // --------- Residue / Chain bookkeeping -----------
                    let res_sn = fields[c_res_sn].parse::<u32>().unwrap_or(0);
                    let chain_id = fields[c_chain];
                    let res_key = (chain_id.to_string(), res_sn);

                    // Residues
                    let r_i = *res_idx.entry(res_key.clone()).or_insert_with(|| {
                        let idx = residues.len();
                        residues.push(ResidueGeneric {
                            serial_number: res_sn,
                            res_type: ResidueType::from_str(fields[c_res]),
                            atom_sns: Vec::new(),
                            end: ResidueEnd::Internal, // We update this after.
                        });
                        idx
                    });
                    residues[r_i].atom_sns.push(serial_number);

                    // Chains
                    let c_i = *chain_idx.entry(chain_id.to_string()).or_insert_with(|| {
                        let idx = chains.len();
                        chains.push(ChainGeneric {
                            id: chain_id.to_string(),
                            residue_sns: Vec::new(),
                            atom_sns: Vec::new(),
                        });
                        idx
                    });
                    chains[c_i].atom_sns.push(serial_number);
                    if !chains[c_i].residue_sns.contains(&res_sn) {
                        chains[c_i].residue_sns.push(res_sn);
                    }

                    i += 1;
                }
                continue; // outer while will handle terminator line
            }

            if line.starts_with('_') {
                if let Some((tag, val)) = line.split_once(char::is_whitespace) {
                    metadata.insert(
                        tag.to_string(),
                        val.trim_matches('\'').to_string().trim().to_string(),
                    );
                } else {
                    metadata.insert(line.to_string().trim().to_string(), String::new());
                }
            }

            i += 1; // advance to next top-level line
        }

        // Populate the residue end, now that we know when the last non-het one is.
        {
            let mut last_non_het = 0;
            for (i, res) in residues.iter().enumerate() {
                match res.res_type {
                    ResidueType::AminoAcid(_) => last_non_het = i,
                    _ => break,
                }
            }

            for (i, res) in residues.iter_mut().enumerate() {
                let mut end = ResidueEnd::Internal;

                // Match arm won't work due to non-constant arms, e.g. non_hetero?
                if i == 0 {
                    end = ResidueEnd::NTerminus;
                } else if i == last_non_het {
                    end = ResidueEnd::CTerminus;
                }

                match res.res_type {
                    ResidueType::AminoAcid(_) => (),
                    _ => end = ResidueEnd::Hetero,
                }

                res.end = end;
            }
        }

        let ident = metadata
            .get("_struct.entry_id")
            .or_else(|| metadata.get("_entry.id"))
            .cloned()
            .unwrap_or_else(|| "UNKNOWN".to_string())
            .trim()
            .to_owned();

        // let mut cursor = Cursor::new(text);

        // let ss_load = Instant::now();
        // todo: Integraet this so it's not taking a second line loop through the whole file.
        // todo: It'll be faster this way.
        // todo: Regardless of that, this SS loading is going very slowly. Fix it.
        // let (secondary_structure, experimental_method) = load_ss_method(&mut cursor)?;

        // let ss_load_time = ss_load.elapsed();
        let secondary_structure = Vec::new();

        Ok(Self {
            ident,
            metadata,
            atoms,
            chains,
            residues,
            secondary_structure,
            experimental_method,
        })
    }
}

#[derive(Clone, Debug)]
/// See note elsewhere regarding serial numbers vs indices: In your downstream applications, you may
/// wish to convert sns to indices, for faster operations.
pub struct BackboneSS {
    /// Atom serial numbers.
    pub start_sn: u32,
    pub end_sn: u32,
    pub sec_struct: SecondaryStructure,
}
#[derive(Clone, Copy, PartialEq, Debug)]
/// The method used to find a given molecular structure. This data is present in mmCIF files
/// as the `_exptl.method` field.
pub enum ExperimentalMethod {
    XRayDiffraction,
    ElectronDiffraction,
    NeutronDiffraction,
    /// i.e. Cryo-EM
    ElectronMicroscopy,
    SolutionNmr,
}

impl ExperimentalMethod {
    /// E.g. for displaying in the space-constrained UI.
    pub fn to_str_short(&self) -> String {
        match self {
            Self::XRayDiffraction => "X-ray",
            Self::NeutronDiffraction => "ND",
            Self::ElectronDiffraction => "ED",
            Self::ElectronMicroscopy => "EM",
            Self::SolutionNmr => "NMR",
        }
        .to_owned()
    }
}

impl Display for ExperimentalMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let val = match self {
            Self::XRayDiffraction => "X-Ray diffraction",
            Self::NeutronDiffraction => "Neutron diffraction",
            Self::ElectronDiffraction => "Electron diffraction",
            Self::ElectronMicroscopy => "Electron microscopy",
            Self::SolutionNmr => "Solution NMR",
        };
        write!(f, "{val}")
    }
}

impl FromStr for ExperimentalMethod {
    type Err = io::Error;

    /// Parse an mmCIF‐style method string into an ExperimentalMethod.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_lowercase();
        let s = normalized.trim();
        let method = match s {
            "x-ray diffraction" => ExperimentalMethod::XRayDiffraction,
            "neutron diffraction" => ExperimentalMethod::NeutronDiffraction,
            "electron diffraction" => ExperimentalMethod::ElectronDiffraction,
            "electron microscopy" => ExperimentalMethod::ElectronMicroscopy,
            "solution nmr" => ExperimentalMethod::SolutionNmr,
            other => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Error parsing experimental method: {other}"),
                ));
            }
        };
        Ok(method)
    }
}

pub struct ParserOptions {}
pub fn parse_mmcif(sdf: &str, options: Option<&ParserOptions>) -> MmCif {
    _parse_mmcif(sdf, options, RustLogger)
}

pub fn _parse_mmcif(
    mmcif_str: &str,
    _options: Option<&ParserOptions>,
    _logger: impl Logger,
) -> MmCif {
    let mmcif = MmCif::new(mmcif_str);

    match mmcif {
        Ok(mmcif) => mmcif,
        Err(err) => {
            _logger.error(&format!("Error parsing MMCIF: {}", err));
            panic!("Error parsing MMCIF: {}", err)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chain {
    pub id: String,
    pub residues: Vec<Residue>,

    ss_cache: Option<Vec<SecondaryStructure>>,
    #[serde(skip)]
    ss: OnceCell<Vec<SecondaryStructure>>,
}

impl Chain {
    pub fn init_ss(&mut self) {
        let calculator = SecondaryStructureCalculator::new();
        self.ss_cache = Some(calculator.compute_secondary_structure(&self.residues));
    }

    pub fn get_ss(&self) -> &Vec<SecondaryStructure> {
        if let Some(cache) = self.ss_cache.as_ref() {
            return &cache;
        }
        self.ss.get_or_init(|| {
            let calculator = SecondaryStructureCalculator::new();
            calculator.compute_secondary_structure(&self.residues)
        })
    }

    pub fn new(id: String, residues: Vec<Residue>) -> Self {
        Self {
            id,
            residues,
            ss_cache: None,
            ss: OnceCell::new(), // 初始化私有缓存
        }
    }
}
