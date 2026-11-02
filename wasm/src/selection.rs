//! Checked references into a separate CK WASM module; no shared Rust pointers.
use cosmol_viewer_core::display::{Display, input::*};
use cosmol_viewer_core::styles::Representation;
use cosmolkit as ck;
use serde::de::DeserializeOwned;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen(module = "@cosmol-studio/cosmolkit")]
extern "C" {
    #[wasm_bindgen(js_name=Molecule,typescript_type="import(\"@cosmol-studio/cosmolkit\").Molecule")]
    pub type CkMolecule;
    #[wasm_bindgen(js_name=Protein,typescript_type="import(\"@cosmol-studio/cosmolkit\").Protein")]
    pub type CkProtein;
    #[wasm_bindgen(js_name=Element,typescript_type="import(\"@cosmol-studio/cosmolkit\").Element")]
    pub type CkElement;
}
#[wasm_bindgen]
extern "C" {
    // This external union is checked with the real imported constructors before reading.
    #[wasm_bindgen(
        typescript_type = "import(\"@cosmol-studio/cosmolkit\").Molecule | import(\"@cosmol-studio/cosmolkit\").Protein"
    )]
    pub type ScientificInput;
}
fn error(e: impl std::fmt::Display) -> JsValue {
    js_sys::TypeError::new(&e.to_string()).into()
}
fn decode<T: DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(error)
}
/// Drop temporary CK getter handles even if a later foreign call throws.
#[derive(Default)]
struct Reader {
    owned: RefCell<Vec<JsValue>>,
}
impl Reader {
    fn own(&self, v: JsValue) -> JsValue {
        if v.is_object() && js_sys::Reflect::get(&v, &"free".into()).is_ok_and(|f| f.is_function())
        {
            self.owned.borrow_mut().push(v.clone());
        }
        v
    }
    fn get(&self, object: &JsValue, name: &str) -> Result<JsValue, JsValue> {
        let v = js_sys::Reflect::get(object, &name.into())?;
        let v = if let Some(f) = v.dyn_ref::<js_sys::Function>() {
            f.call0(object)?
        } else {
            v
        };
        Ok(self.own(v))
    }
    fn list(&self, v: JsValue) -> Result<Vec<JsValue>, JsValue> {
        let iter = js_sys::try_iter(&v)?.ok_or_else(|| error("expected a CK entity sequence"))?;
        iter.map(|v| v.map(|v| self.own(v))).collect()
    }
    fn call(&self, object: &JsValue, name: &str, args: &js_sys::Array) -> Result<JsValue, JsValue> {
        let f = js_sys::Reflect::get(object, &name.into())?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| error("expected CK method"))?;
        Ok(self.own(f.apply(object, args)?))
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        for value in self.owned.get_mut().drain(..).rev() {
            if let Ok(f) = js_sys::Reflect::get(&value, &"free".into())
                .and_then(|f| f.dyn_into::<js_sys::Function>())
            {
                let _ = f.call0(&value);
            }
        }
    }
}
impl ScientificInput {
    pub(crate) fn capture(&self) -> Result<SourceInput, JsValue> {
        let reader = Reader::default();
        if self.is_instance_of::<CkMolecule>() {
            molecule(&reader, self.as_ref()).map(SourceInput::Molecule)
        } else if self.is_instance_of::<CkProtein>() {
            protein(&reader, self.as_ref()).map(SourceInput::Protein)
        } else {
            Err(error("expected a cosmolkit.Molecule or cosmolkit.Protein"))
        }
    }
}
fn molecule(r: &Reader, source: &JsValue) -> Result<MoleculeInput, JsValue> {
    let elements = r
        .list(r.get(source, "atoms")?)?
        .iter()
        .map(|a| {
            let n: u8 = decode(r.get(a, "atomicNumber")?)?;
            ck::Element::from_atomic_number(n).ok_or_else(|| error("invalid element"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bonds = r
        .list(r.get(source, "bonds")?)?
        .iter()
        .map(|b| {
            let code: i64 = decode(r.get(b, "orderCode")?)?;
            Ok(BondInput {
                begin: decode(r.get(b, "begin")?)?,
                end: decode(r.get(b, "end")?)?,
                order: ck::BondOrder::from_rdkit_code(code)
                    .ok_or_else(|| error("invalid bond order"))?,
                aromatic: decode(r.get(b, "isAromatic")?)?,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    let conformers = r
        .list(r.get(source, "conformers3d")?)?
        .iter()
        .map(|c| {
            Ok(ConformerInput {
                id: decode(r.get(c, "id")?)?,
                coordinates: decode(r.get(c, "coordinates")?)?,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    let coordinates_2d = decode(r.get(source, "coordinates2d")?)?;
    let atom_weights = (0..elements.len())
        .map(|i| {
            let args = js_sys::Array::of2(&JsValue::from_f64(i as f64), &"WEIGHT".into());
            decode(r.call(source, "atomPropertyDouble", &args)?)
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    let properties = r.get(source, "properties")?;
    let fields: Vec<(String, String)> = decode(r.get(&properties, "sdfDataFields")?)?;
    let field = |name: &str| {
        fields
            .iter()
            .rev()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    let input = MoleculeInput {
        elements,
        bonds,
        conformers,
        coordinates_2d,
        atom_weights,
        dummy_weights: field("DUMMY_WEIGHTS"),
        dummy_indices: field("DUMMY_ATOM_INDICES"),
    };
    for c in &input.conformers {
        for p in &c.coordinates {
            if p.iter().any(|&v| !(v as f32).is_finite()) {
                return Err(error("coordinates must fit finite f32 values"));
            }
        }
    }
    if let Some(xy) = &input.coordinates_2d {
        if xy.iter().flatten().any(|&v| !(v as f32).is_finite()) {
            return Err(error("coordinates must fit finite f32 values"));
        }
    }
    Ok(input)
}
fn protein(r: &Reader, source: &JsValue) -> Result<ProteinInput, JsValue> {
    let mut chains = Vec::new();
    for chain in r.list(r.get(source, "chains")?)? {
        let mut residues = Vec::new();
        for residue in r.list(r.get(&chain, "residues")?)? {
            let name: String = decode(r.get(&residue, "name")?)?;
            let row = r.get(&residue, "row")?;
            let src = r.get(&row, "source")?;
            let seq = r.get(&src, "seqId")?;
            let seq_id = if seq.is_null() || seq.is_undefined() {
                None
            } else {
                Some((
                    decode(r.get(&seq, "seqNum")?)?,
                    decode(r.get(&seq, "insCode")?)?,
                ))
            };
            let mut atoms = Vec::new();
            for atom in r.list(r.get(&residue, "atoms")?)? {
                let row = r.get(&atom, "row")?;
                let p: [f64; 3] = decode(r.get(&atom, "position")?)?;
                if p.iter().any(|&v| !(v as f32).is_finite()) {
                    return Err(error("coordinates must fit finite f32 values"));
                }
                let altloc = r.get(&row, "altloc")?;
                let atomname = r.get(&atom, "name")?;
                atoms.push(ProteinAtomInput {
                    id: ck::BioAtomId::new(decode(r.get(&atom, "id")?)?),
                    name: decode::<String>(r.get(&atomname, "asStr")?)?
                        .trim()
                        .to_owned(),
                    position: p.map(|v| v as f32).into(),
                    source_position: p,
                    atomic_number: decode(r.get(&r.get(&atom, "element")?, "atomicNumber")?)?,
                    alt_loc: if altloc.is_undefined() || altloc.is_null() {
                        None
                    } else {
                        Some(decode(r.get(&altloc, "value")?)?)
                    },
                    occupancy: decode(r.get(&row, "occupancy")?)?,
                    b_factor: decode(r.get(&row, "bIso")?)?,
                });
            }
            residues.push(ProteinResidueInput {
                id: ck::BioResidueId::new(decode(r.get(&residue, "id")?)?),
                residue_type: ck::find_residue_info(&name).code,
                name,
                atoms,
                seq_id,
                label_seq_id: decode(r.get(&src, "labelSeqId")?)?,
            });
        }
        chains.push(residues);
    }
    let input_format = match decode::<u32>(r.get(source, "inputFormat")?)? {
        2 => ck::BioCoordinateFormat::Pdb,
        3 => ck::BioCoordinateFormat::Mmcif,
        _ => return Err(error("expected a PDB or mmCIF protein source")),
    };
    Ok(ProteinInput {
        chains,
        input_format,
    })
}
struct Snapshot {
    owner: JsValue,
    input: SourceInput,
}
#[wasm_bindgen]
pub struct Selector {
    snapshot: Rc<Snapshot>,
}
#[wasm_bindgen]
pub struct Selection {
    snapshot: Rc<Snapshot>,
    kind: EntityKind,
    ids: Vec<usize>,
}
impl crate::input::NativeOutput for Selector {}
impl crate::input::NativeOutput for Selection {}
#[wasm_bindgen]
pub fn select(source: &ScientificInput) -> Result<Selector, JsValue> {
    Ok(Selector {
        snapshot: Rc::new(Snapshot {
            owner: AsRef::<JsValue>::as_ref(source).clone(),
            input: source.capture()?,
        }),
    })
}
impl Selector {
    fn entities(&self, kind: EntityKind) -> Result<Selection, JsValue> {
        Ok(Selection {
            snapshot: self.snapshot.clone(),
            kind,
            ids: self.snapshot.input.ids(kind).map_err(error)?,
        })
    }
}
#[cosmol_viewer_derive::checked_wasm]
impl Selector {
    pub fn atoms(&self) -> Result<Selection, JsValue> {
        self.entities(EntityKind::Atoms)
    }
    pub fn bonds(&self) -> Result<Selection, JsValue> {
        self.entities(EntityKind::Bonds)
    }
    pub fn residues(&self) -> Result<Selection, JsValue> {
        self.entities(EntityKind::Residues)
    }
}
#[cosmol_viewer_derive::checked_wasm]
impl Selection {
    pub fn element(&self, element: &CkElement) -> Result<Self, JsValue> {
        if !element.is_instance_of::<CkElement>() {
            return Err(error("expected cosmolkit.Element"));
        }
        let r = Reader::default();
        let number = decode(r.get(element.as_ref(), "atomicNumber")?)?;
        let element =
            ck::Element::from_atomic_number(number).ok_or_else(|| error("invalid element"))?;
        let mut ids = self.ids.clone();
        self.snapshot
            .input
            .filter_element(self.kind, &mut ids, element)
            .map_err(error)?;
        Ok(Self {
            snapshot: self.snapshot.clone(),
            kind: self.kind,
            ids,
        })
    }
    pub fn aromatic(&self) -> Result<Self, JsValue> {
        let mut ids = self.ids.clone();
        self.snapshot
            .input
            .filter_aromatic(self.kind, &mut ids)
            .map_err(error)?;
        Ok(Self {
            snapshot: self.snapshot.clone(),
            kind: self.kind,
            ids,
        })
    }
    pub fn ids(&self) -> Vec<usize> {
        self.ids.clone()
    }
    pub fn len(&self) -> usize {
        self.ids.len()
    }
}
impl Selection {
    pub(crate) fn project(
        &self,
        source: &ScientificInput,
        style: Representation,
    ) -> Result<Display, JsValue> {
        if !js_sys::Object::is(source.as_ref(), &self.snapshot.owner) {
            return Err(error("selection belongs to a different source object"));
        }
        if source.capture()? != self.snapshot.input {
            return Err(error(
                "source rendering data changed since selection; create a new selection",
            ));
        }
        self.snapshot
            .input
            .project(self.kind, &self.ids, style)
            .map_err(|e| JsValue::from(js_sys::Error::new(&e.to_string())))
    }
}
