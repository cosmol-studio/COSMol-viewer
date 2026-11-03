//! Import foreign CK objects once; selection and projection run in core Rust.
use cosmol_viewer_core::selection as native;
use cosmolkit as ck;
use std::{
    cell::RefCell,
    collections::HashSet,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, convert::TryFromJsValue, prelude::*};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name=Molecule,typescript_type="import(\"@cosmol-studio/cosmolkit\").Molecule")]
    pub type CkMolecule;
    #[wasm_bindgen(js_name=Protein,typescript_type="import(\"@cosmol-studio/cosmolkit\").Protein")]
    pub type CkProtein;
    #[wasm_bindgen(js_name=Element,typescript_type="import(\"@cosmol-studio/cosmolkit\").Element")]
    pub type CkElement;
}

// Only the npm entry point supplies CK constructors. Notebook transport uses
// serialized core scenes and never loads CK JavaScript or calls these adapters.
struct CkTypes {
    molecule: js_sys::Function,
    protein: js_sys::Function,
    element: js_sys::Function,
}
thread_local! {
    static CK_TYPES: RefCell<Option<CkTypes>> = const { RefCell::new(None) };
}
#[wasm_bindgen(js_name = __setCkTypes)]
pub fn set_ck_types(
    molecule: js_sys::Function,
    protein: js_sys::Function,
    element: js_sys::Function,
) {
    CK_TYPES.with(|types| {
        *types.borrow_mut() = Some(CkTypes {
            molecule,
            protein,
            element,
        })
    });
}

struct ViewerTypes {
    selection: js_sys::Function,
    sphere: js_sys::Function,
    stick: js_sys::Function,
    style: js_sys::Function,
}
thread_local! { static VIEWER_TYPES: RefCell<Option<ViewerTypes>> = const { RefCell::new(None) }; }
#[wasm_bindgen(js_name = __setViewerTypes)]
pub fn set_viewer_types(
    selection: js_sys::Function,
    sphere: js_sys::Function,
    stick: js_sys::Function,
    style: js_sys::Function,
) {
    VIEWER_TYPES.with(|types| {
        *types.borrow_mut() = Some(ViewerTypes {
            selection,
            sphere,
            stick,
            style,
        })
    });
}
fn viewer_instance(
    value: &JsValue,
    constructor: impl FnOnce(&ViewerTypes) -> &js_sys::Function,
) -> Result<bool, JsValue> {
    VIEWER_TYPES.with(|types| {
        let types = types.borrow();
        let types = types
            .as_ref()
            .ok_or_else(|| error("Initialize through the viewer npm entry point"))?;
        let class = constructor(types);
        let has_instance = js_sys::Reflect::get(class, &js_sys::Symbol::has_instance())?
            .dyn_into::<js_sys::Function>()?;
        Ok(has_instance.call1(class, value)?.as_bool().unwrap_or(false))
    })
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Style")]
    pub type StyleInput;
}
impl StyleInput {
    pub(crate) fn representation(
        &self,
    ) -> Result<cosmol_viewer_core::styles::Representation, JsValue> {
        if !viewer_instance(self.as_ref(), |types| &types.style)? {
            return Err(error("expected Style"));
        }
        let style = crate::styles::Style::try_from_js_value(call(self.as_ref(), "cloneStyle")?)
            .map_err(|_| error("expected live Style"))?;
        Ok(style.inner)
    }
}
fn ck_instance(
    value: &JsValue,
    constructor: impl FnOnce(&CkTypes) -> &js_sys::Function,
) -> Result<bool, JsValue> {
    CK_TYPES.with(|types| {
        let types = types.borrow();
        let types = types.as_ref().ok_or_else(|| {
            error("CK object APIs require the @cosmol-studio/cosmol-viewer npm entry point")
        })?;
        let class = constructor(types);
        let has_instance = js_sys::Reflect::get(class, &js_sys::Symbol::has_instance())?
            .dyn_into::<js_sys::Function>()?;
        Ok(has_instance.call1(class, value)?.as_bool().unwrap_or(false))
    })
}
#[wasm_bindgen]
extern "C" {
    // This external union is checked with the real imported constructors before reading.
    #[wasm_bindgen(
        typescript_type = "import(\"@cosmol-studio/cosmolkit\").Molecule | import(\"@cosmol-studio/cosmolkit\").Protein"
    )]
    pub type ScientificInput;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        typescript_type = "import(\"@cosmol-studio/cosmolkit\").Molecule | import(\"@cosmol-studio/cosmolkit\").Protein | Selection | Sphere | Stick"
    )]
    pub type SceneInput;
}
fn error(e: impl std::fmt::Display) -> JsValue {
    js_sys::TypeError::new(&e.to_string()).into()
}
fn call(value: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    js_sys::Reflect::get(value, &name.into())?
        .dyn_into::<js_sys::Function>()?
        .call0(value)
}
pub(crate) enum NativeSource {
    Molecule(ck::Molecule),
    Protein(ck::Protein),
}
pub(crate) struct ImportedSource {
    owner: JsValue,
    pub(crate) native: NativeSource,
}
thread_local! {
    static IMPORTS: RefCell<Vec<Weak<ImportedSource>>> = const { RefCell::new(Vec::new()) };
}
impl ScientificInput {
    pub(crate) fn import(&self) -> Result<Rc<ImportedSource>, JsValue> {
        import(self.as_ref())
    }
}
fn import(value: &JsValue) -> Result<Rc<ImportedSource>, JsValue> {
    let existing = IMPORTS.with(|imports| {
        imports
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .find(|source| js_sys::Object::is(&source.owner, value))
    });
    if let Some(source) = existing {
        return Ok(source);
    }
    let molecule = ck_instance(value, |types| &types.molecule)?;
    if !molecule && !ck_instance(value, |types| &types.protein)? {
        return Err(error("expected a cosmolkit.Molecule or cosmolkit.Protein"));
    }
    let bytes = call(value, "toTransferBinary")?
        .dyn_into::<js_sys::Uint8Array>()?
        .to_vec();
    let native = if molecule {
        NativeSource::Molecule(ck::Molecule::from_transfer_binary(&bytes).map_err(error)?)
    } else {
        NativeSource::Protein(ck::Protein::from_transfer_binary(&bytes).map_err(error)?)
    };
    let source = Rc::new(ImportedSource {
        owner: value.clone(),
        native,
    });
    IMPORTS.with(|imports| {
        let mut imports = imports.borrow_mut();
        imports.retain(|entry| entry.strong_count() > 0);
        imports.push(Rc::downgrade(&source));
    });
    Ok(source)
}
#[derive(Clone)]
enum Ids {
    MoleculeAtoms(Vec<ck::AtomId>),
    MoleculeBonds(Vec<ck::BondId>),
    ProteinAtoms(Vec<ck::BioAtomId>),
    ProteinResidues(Vec<ck::BioResidueId>),
}
enum NativeSelection<'a> {
    MoleculeAtoms(native::MoleculeAtomSelection<'a>),
    MoleculeBonds(native::MoleculeBondSelection<'a>),
    ProteinAtoms(native::ProteinAtomSelection<'a>),
    ProteinResidues(native::ProteinResidueSelection<'a>),
}
#[wasm_bindgen]
pub struct Selector {
    source: Rc<ImportedSource>,
}
#[derive(Clone)]
#[wasm_bindgen]
pub struct Selection {
    source: Rc<ImportedSource>,
    ids: Ids,
}
impl crate::input::NativeOutput for Selector {}
impl crate::input::NativeOutput for Selection {}
#[wasm_bindgen]
pub fn select(source: &ScientificInput) -> Result<Selector, JsValue> {
    Ok(Selector {
        source: source.import()?,
    })
}
#[cosmol_viewer_derive::checked_wasm]
impl Selector {
    pub fn atoms(&self) -> Selection {
        let ids = match &self.source.native {
            NativeSource::Molecule(mol) => {
                Ids::MoleculeAtoms(native::select(mol).atoms().ids().to_vec())
            }
            NativeSource::Protein(protein) => {
                Ids::ProteinAtoms(native::select(protein).atoms().ids().to_vec())
            }
        };
        Selection {
            source: self.source.clone(),
            ids,
        }
    }
    pub fn bonds(&self) -> Result<Selection, JsValue> {
        let NativeSource::Molecule(mol) = &self.source.native else {
            return Err(error("protein selections cannot contain bonds"));
        };
        Ok(Selection {
            source: self.source.clone(),
            ids: Ids::MoleculeBonds(native::select(mol).bonds().ids().to_vec()),
        })
    }
    pub fn residues(&self) -> Result<Selection, JsValue> {
        let NativeSource::Protein(protein) = &self.source.native else {
            return Err(error("molecule selections cannot contain residues"));
        };
        Ok(Selection {
            source: self.source.clone(),
            ids: Ids::ProteinResidues(native::select(protein).residues().ids().to_vec()),
        })
    }
}
fn subset<'a, S, I: Copy + Eq + std::hash::Hash>(
    selection: native::Selection<'a, S, I>,
    ids: &[I],
) -> native::Selection<'a, S, I> {
    let ids: HashSet<_> = ids.iter().copied().collect();
    selection.filter(|id| ids.contains(id))
}
impl Selection {
    fn native(&self) -> NativeSelection<'_> {
        match (&self.source.native, &self.ids) {
            (NativeSource::Molecule(mol), Ids::MoleculeAtoms(ids)) => {
                NativeSelection::MoleculeAtoms(subset(native::select(mol).atoms(), ids))
            }
            (NativeSource::Molecule(mol), Ids::MoleculeBonds(ids)) => {
                NativeSelection::MoleculeBonds(subset(native::select(mol).bonds(), ids))
            }
            (NativeSource::Protein(protein), Ids::ProteinAtoms(ids)) => {
                NativeSelection::ProteinAtoms(subset(native::select(protein).atoms(), ids))
            }
            (NativeSource::Protein(protein), Ids::ProteinResidues(ids)) => {
                NativeSelection::ProteinResidues(subset(native::select(protein).residues(), ids))
            }
            _ => unreachable!("selection kind belongs to its native source"),
        }
    }
}
#[cosmol_viewer_derive::checked_wasm]
impl Selection {
    /// Copy the selection handle while sharing the same imported source.
    pub fn clone_selection(&self) -> Self {
        self.clone()
    }
    pub fn element(&self, element: &CkElement) -> Result<Self, JsValue> {
        if !ck_instance(element.as_ref(), |types| &types.element)? {
            return Err(error("expected cosmolkit.Element"));
        }
        let number: u8 = serde_wasm_bindgen::from_value(js_sys::Reflect::get(
            element.as_ref(),
            &"atomicNumber".into(),
        )?)
        .map_err(error)?;
        let element =
            ck::Element::from_atomic_number(number).ok_or_else(|| error("invalid element"))?;
        let ids = match self.native() {
            NativeSelection::MoleculeAtoms(s) => {
                Ids::MoleculeAtoms(s.element(element).ids().to_vec())
            }
            NativeSelection::ProteinAtoms(s) => {
                Ids::ProteinAtoms(s.element(element).ids().to_vec())
            }
            _ => return Err(error("element filtering requires an atom selection")),
        };
        Ok(Self {
            source: self.source.clone(),
            ids,
        })
    }
    pub fn aromatic(&self) -> Result<Self, JsValue> {
        let NativeSelection::MoleculeBonds(selection) = self.native() else {
            return Err(error(
                "aromatic filtering requires a molecule bond selection",
            ));
        };
        Ok(Self {
            source: self.source.clone(),
            ids: Ids::MoleculeBonds(selection.aromatic().ids().to_vec()),
        })
    }
    pub fn ids(&self) -> Vec<usize> {
        match &self.ids {
            Ids::MoleculeAtoms(ids) => ids.iter().map(|id| id.index()).collect(),
            Ids::MoleculeBonds(ids) => ids.iter().map(|id| id.index()).collect(),
            Ids::ProteinAtoms(ids) => ids.iter().map(|id| id.index()).collect(),
            Ids::ProteinResidues(ids) => ids.iter().map(|id| id.index()).collect(),
        }
    }
    pub fn len(&self) -> usize {
        match &self.ids {
            Ids::MoleculeAtoms(ids) => ids.len(),
            Ids::MoleculeBonds(ids) => ids.len(),
            Ids::ProteinAtoms(ids) => ids.len(),
            Ids::ProteinResidues(ids) => ids.len(),
        }
    }
}
impl SceneInput {
    pub(crate) fn with_native(
        &self,
        operation: impl FnOnce(&dyn cosmol_viewer_core::display::SceneItem) -> Result<(), JsValue>,
    ) -> Result<Option<Rc<ImportedSource>>, JsValue> {
        let value: &JsValue = self.as_ref();
        if viewer_instance(value, |types| &types.sphere)? {
            let shape = crate::shapes::Sphere::try_from_js_value(call(value, "cloneShape")?)
                .map_err(|_| error("expected live Sphere"))?;
            operation(&shape.inner)?;
            Ok(None)
        } else if viewer_instance(value, |types| &types.stick)? {
            let shape = crate::shapes::Stick::try_from_js_value(call(value, "cloneShape")?)
                .map_err(|_| error("expected live Stick"))?;
            operation(&shape.inner)?;
            Ok(None)
        } else if viewer_instance(value, |types| &types.selection)? {
            // Official wasm-bindgen conversion consumes its argument. Convert an
            // explicit clone so callers can reuse the original selection handle.
            let selection = Selection::try_from_js_value(call(value, "cloneSelection")?)
                .map_err(|_| error("expected a live Selection"))?;
            match selection.native() {
                NativeSelection::MoleculeAtoms(s) => operation(&s),
                NativeSelection::MoleculeBonds(s) => operation(&s),
                NativeSelection::ProteinAtoms(s) => operation(&s),
                NativeSelection::ProteinResidues(s) => operation(&s),
            }?;
            Ok(Some(selection.source))
        } else {
            let source = import(value)?;
            match &source.native {
                NativeSource::Molecule(mol) => operation(mol),
                NativeSource::Protein(protein) => operation(protein),
            }?;
            Ok(Some(source))
        }
    }
}
