# COSMolKit and COSMol Viewer API Boundary Plan

Status: Proposed API contract, not implemented.

This plan covers public APIs and observable behavior only. COSMolKit owns persistent scientific objects and selections. COSMol Viewer owns their visual projections, styles, and display state. Users pass CK objects directly to a scene without creating a second chemical object in the viewer.

## Ownership boundaries

| Concern | Owner |
| --- | --- |
| `Molecule`, `BioStructure`, topology, coordinates, and scientific properties | COSMolKit |
| Atom, bond, and residue selections and their source identity | COSMolKit |
| `element`, `aromatic`, `protein`, `chain`, `within`, and entity conversions such as `residues().atoms()` | COSMolKit |
| `Scene`, render styles, visibility, and other display state | COSMol Viewer |
| Render geometry and GPU resources derived from a source and selection | COSMol Viewer |

The viewer consumes a source object, selected entities, and a render style. It does not implement chemical selection queries or become the authoritative owner of a duplicate molecule or protein structure.

## Rust API

The following examples describe the target API rather than currently available signatures. `show` is fallible so invalid source or style combinations can be reported explicitly.

```rust
let mol = ck::Molecule::from_sdf(sdf)?;

let oxygens = mol.atoms().element(ck::Element::O);
let aromatic = mol.bonds().aromatic();

let mut scene = cv::Scene::new();

scene.show(&mol, mol.atoms(), cv::BallAndStick::default())?;
scene.show(&mol, &oxygens, cv::Sphere::default().scale(1.4))?;
scene.show(&mol, &aromatic, cv::Stick::default().color(color))?;
```

`show` accepts a CK selection directly, including a borrowed reusable selection. No conversion into a viewer-owned molecule is required.

Protein visualization uses the same contract:

```rust
let structure = ck::BioStructure::from_mmcif(mmcif)?;
// A CK selection of ligand atoms belonging to this structure.
let ligand = ligand_selection;

let pocket = structure
    .atoms()
    .protein()
    .within(6.0, &ligand)
    .residues()
    .atoms();

scene.show(
    &structure,
    structure.residues().protein(),
    cv::Cartoon::default(),
)?;
scene.show(&structure, &pocket, cv::Stick::default())?;
scene.show(&structure, &ligand, cv::BallAndStick::default())?;
```

CK evaluates the pocket query. The viewer receives the resulting selected entities and derives the requested geometry from the source.

## Python API

```python
import cosmolkit as ck
import cosmol_viewer as cv

mol = ck.Molecule.from_sdf(sdf)
oxygen = mol.atoms().element(ck.Element.O)

scene = cv.Scene()
scene.show(mol, style=cv.BallAndStick())
scene.show(mol, oxygen, style=cv.Sphere(scale=1.4))
```

The target call forms are:

```python
scene.show(source, style=style)
scene.show(source, selection, style=style)
scene.show(source, selection, style)
```

Omitting `selection` means all source entities appropriate for the requested style, not an implicit chemical filter. For example, `Cartoon` does not implicitly call `protein()`; select protein residues explicitly when that restriction is wanted.

Python accepts CK objects directly. The long-term API does not require `cv.Molecule.from_cosmolkit(mol)` or another viewer-owned chemical wrapper.

## Display behavior

Each successful `show` call adds an independent display associating the source, selection, and style. It does not replace earlier displays of the same source.

- One CK object can contribute any number of displays without duplicating its scientific structure for each display.
- Overlapping selections are allowed. Styling one display does not change the CK object or another display.
- A protein can appear simultaneously as a cartoon, pocket sticks, and ligand ball-and-stick geometry.
- Displays belong to the scene. CK objects do not store viewer representations or render styles.
- Styles consume compatible entity kinds: for example, spheres use atoms, sticks can use bonds, and cartoons use suitable residues and connectivity. Unsupported combinations produce an explicit error rather than silently changing the selection.
- An empty compatible selection produces no geometry and is not an error.
- A rejected `show` call adds no display and leaves existing scene state unchanged.

Keeping several displays does not imply a second editable scientific model. Derived geometry, GPU buffers, and transport payloads are permitted; they are visual data, not an independently mutable molecule or protein API. This contract does not promise zero-copy GPU upload or IPC.

## Selection identity and validity

Every selection belongs to a specific CK source value. Bare indices are not sufficient to establish that relationship.

```rust
let mol1 = ck::Molecule::from_sdf(sdf1)?;
let mol2 = ck::Molecule::from_sdf(sdf2)?;
let oxygen = mol1.atoms().element(ck::Element::O);

// Must be rejected: oxygen belongs to mol1, not mol2.
scene.show(&mol2, &oxygen, cv::Sphere::default())?;
```

Rejection should happen at compile time where the public types can enforce it. Otherwise, Rust must return a clear error and Python must raise a clear exception identifying the source mismatch. Matching entity counts, indices, or contents must not make an unrelated source acceptable.

Rust lifetimes can prevent a selection from outliving its borrowed source. A lifetime alone does not prove that two references point to the same source value; source identity must also be enforced.

Value-style topology changes create a new source value:

```rust
let oxygen = mol.atoms().element(ck::Element::O);
let hydrogenated = mol.with_hydrogens();

// Must be rejected even if some atom indices still match.
scene.show(&hydrogenated, &oxygen, cv::Sphere::default())?;

// Recompute a selection against the new source.
scene.show(
    &hydrogenated,
    hydrogenated.atoms().element(ck::Element::O),
    cv::Sphere::default(),
)?;
```

The viewer must not silently remap, rebind, or reinterpret a selection against another topology value.

## Source lifetime and updates

- Rust displays must not outlive their source; the API must enforce valid borrowing or shared ownership.
- Python scenes keep their sources alive while displays reference them. Deleting a local source variable does not invalidate a display.
- Adding a display does not mutate its source, topology, coordinates, properties, or selections.
- Replacing a local variable with a newly derived CK value does not retarget existing displays or selections.
- `show` does not establish an automatic source-mutation subscription. Live scientific updates require a separately specified explicit update contract.

## Future API boundaries

Trajectories, surfaces, and property coloring must preserve the same division: CK supplies scientific data and source-aware selections; the viewer supplies visual projections and display state.

This plan does not define implementation types, storage layouts, selection algorithms, transport protocols, display editing methods, or migration schedules. It does not change the existing `Viewer.render`, `Viewer.update`, or `Viewer.keep_alive` lifecycle contract.
