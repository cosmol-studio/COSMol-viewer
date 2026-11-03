//! Source-borrowing selections, independent of rendering and CK's collection APIs.
//!
//! ```
//! use cosmolkit as ck;
//! use cosmol_viewer_core::selection::select;
//!
//! # fn example(mol: &ck::Molecule, protein: &ck::Protein) {
//! let oxygen = select(mol).atoms().element(ck::Element::O);
//! assert!(oxygen.belongs_to(mol));
//! let protein_oxygen = select(protein).atoms().element(ck::Element::O);
//! let residues = select(protein).residues();
//! # }
//! ```
//!
//! A selection borrows its source, not the temporary selector. It cannot outlive
//! that source or be used across its replacement or mutable modification:
//!
//! ```compile_fail,E0506
//! use cosmolkit as ck;
//! use cosmol_viewer_core::selection::select;
//! fn replace(mut mol: ck::Molecule) {
//!     let atoms = select(&mol).atoms();
//!     mol = ck::Molecule::new();
//!     println!("{}", atoms.len());
//! }
//! ```
//!
//! ```compile_fail,E0502
//! use cosmolkit as ck;
//! use cosmol_viewer_core::selection::select;
//! fn modify(protein: &mut ck::Protein) {
//!     let atoms = select(protein).atoms();
//!     let _mutable_source = &mut *protein;
//!     println!("{}", atoms.len());
//! }
//! ```
//!
//! ```compile_fail,E0597
//! use cosmolkit as ck;
//! use cosmol_viewer_core::selection::select;
//! let atoms;
//! {
//!     let mol = ck::Molecule::new();
//!     atoms = select(&mol).atoms();
//! }
//! println!("{}", atoms.len());
//! ```

use cosmolkit::{AtomId, BioAtomId, BioResidueId, BondId, Element, Molecule, Protein};

/// A source supporting the independent [`select`] entry point.
pub trait Selectable {
    /// The source-specific selector, borrowing this source for `'a`.
    type Selection<'a>
    where
        Self: 'a;

    /// Create a selection context without copying scientific data.
    fn selection(&self) -> Self::Selection<'_>;
}

/// Begin selecting entities from a source, using static dispatch.
pub fn select<S: Selectable + ?Sized>(source: &S) -> S::Selection<'_> {
    source.selection()
}

/// A lightweight context borrowing a scientific source.
pub struct Selector<'a, S: ?Sized> {
    source: &'a S,
}

/// Selected entity IDs tied to one borrowed source object.
///
/// IDs retain source order. Fields and construction are private so IDs cannot
/// be rebound to an unrelated source. No molecule, topology or coordinates are
/// copied. Selections themselves are not serialized; scene displays carry render geometry.
#[derive(Clone)]
pub struct Selection<'a, S: ?Sized, I> {
    source: &'a S,
    ids: Vec<I>,
}

impl<'a, S: ?Sized, I> Selection<'a, S, I> {
    fn new(source: &'a S, ids: Vec<I>) -> Self {
        Self { source, ids }
    }

    /// The original source, with the source's lifetime rather than the selector's.
    pub fn source(&self) -> &'a S {
        self.source
    }

    /// Read selected IDs without allowing them to be modified.
    pub fn ids(&self) -> &[I] {
        &self.ids
    }

    /// Keep selected IDs matching a predicate, preserving source ownership and order.
    pub fn filter(mut self, predicate: impl FnMut(&I) -> bool) -> Self {
        self.ids.retain(predicate);
        self
    }

    /// Number of selected entities.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the selection contains no entities.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether this is the exact borrowed source object.
    ///
    /// Equal contents and cloned CK values do not count as the same object.
    /// Borrowing prevents mutation while this selection remains in use; this
    /// check is not a persistent version ID or an identity for IPC.
    pub fn belongs_to(&self, source: &S) -> bool {
        std::ptr::eq(self.source, source)
    }
}

/// Atom IDs selected from a molecule.
pub type MoleculeAtomSelection<'a> = Selection<'a, Molecule, AtomId>;
/// Bond IDs selected from a molecule.
pub type MoleculeBondSelection<'a> = Selection<'a, Molecule, BondId>;
/// Atom IDs selected from a CK protein (not a viewer render shape).
pub type ProteinAtomSelection<'a> = Selection<'a, Protein, BioAtomId>;
/// Residue IDs selected from a CK protein.
pub type ProteinResidueSelection<'a> = Selection<'a, Protein, BioResidueId>;

impl Selectable for Molecule {
    type Selection<'a> = Selector<'a, Self>;

    fn selection(&self) -> Self::Selection<'_> {
        Selector { source: self }
    }
}

impl Selectable for Protein {
    type Selection<'a> = Selector<'a, Self>;

    fn selection(&self) -> Self::Selection<'_> {
        Selector { source: self }
    }
}

impl<'a> Selector<'a, Molecule> {
    /// Select all atoms in CK topology order.
    pub fn atoms(&self) -> MoleculeAtomSelection<'a> {
        Selection::new(
            self.source,
            self.source
                .topology()
                .atoms
                .iter()
                .map(|atom| atom.id())
                .collect(),
        )
    }

    /// Select all bonds in CK topology order.
    pub fn bonds(&self) -> MoleculeBondSelection<'a> {
        Selection::new(
            self.source,
            self.source.bonds().iter().map(|bond| bond.id()).collect(),
        )
    }
}

impl<'a> Selector<'a, Protein> {
    /// Select all atoms of the already projected CK protein.
    /// No additional model or alternate-location filtering is performed.
    pub fn atoms(&self) -> ProteinAtomSelection<'a> {
        Selection::new(
            self.source,
            self.source.atoms().map(|atom| atom.id()).collect(),
        )
    }

    /// Select all residues of the already projected CK protein.
    pub fn residues(&self) -> ProteinResidueSelection<'a> {
        Selection::new(
            self.source,
            self.source.residues().map(|residue| residue.id()).collect(),
        )
    }
}

impl MoleculeAtomSelection<'_> {
    /// Keep only selected atoms with this CK element. Does not change the source.
    pub fn element(mut self, element: Element) -> Self {
        self.ids.retain(|id| {
            self.source
                .atom(*id)
                .expect("selected source atom")
                .element()
                == element
        });
        self
    }
}

impl MoleculeBondSelection<'_> {
    /// Keep only bonds whose CK aromatic flag is already set.
    /// Does not calculate aromaticity or kekulize the source.
    pub fn aromatic(mut self) -> Self {
        self.ids.retain(|id| {
            self.source
                .bond(*id)
                .expect("selected source bond")
                .is_aromatic()
        });
        self
    }
}

impl ProteinAtomSelection<'_> {
    /// Keep only selected atoms with this CK element. Does not change the source.
    pub fn element(mut self, element: Element) -> Self {
        self.ids.retain(|id| {
            self.source
                .as_bio_structure()
                .atom(*id)
                .expect("selected source atom")
                .element()
                == element
        });
        self
    }
}
