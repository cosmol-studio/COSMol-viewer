# COSMolKit and COSMol Viewer API Boundary Plan

Status: Rust, Python and JavaScript support CK `Molecule` and `Protein` projections. Broader BIO and spatial selections remain proposed.

This plan covers public APIs and observable behavior only. COSMolKit owns persistent scientific objects. An independent selection layer, currently implemented in the Rust viewer, borrows CK objects and evaluates selections without changing CK's collection APIs. COSMol Viewer owns visual projections, styles, and display state. Users pass CK objects directly to a scene without creating a second chemical object in the viewer.

## Ownership boundaries

| Concern | Owner |
| --- | --- |
| `Molecule`, `BioStructure`, topology, coordinates, and scientific properties | COSMolKit |
| Source-aware atom, bond, and residue selection results | Independent selection layer |
| Filtering, spatial queries, and selection entity conversions | Independent selection layer, reading CK's scientific semantics |
| Element, aromaticity, residue classification, coordinates, and structural relationships | COSMolKit |
| `Scene`, render styles, visibility, and other display state | COSMol Viewer |
| Render geometry and GPU resources derived from a source and selection | COSMol Viewer |

The renderer consumes a source object, selected entities, and a render style. The selection layer reads CK's element/aromaticity/classification values rather than reimplementing chemistry. Neither layer becomes the authoritative owner of a duplicate molecule or protein structure.

## Rust API

The molecular example uses the current Rust API. `show` is fallible so invalid source or style combinations can be reported explicitly. The initial selection API supports molecular atoms/bonds and protein atoms/residues, element filtering and aromatic bond filtering. It does not yet implement BIO pocket queries or protein chemical-bond projections.

```rust
let mol = ck::Molecule::from_sdf(sdf)?;

let oxygens = cv::select(&mol).atoms().element(ck::Element::O);
let aromatic = cv::select(&mol).bonds().aromatic();

let mut scene = cv::Scene::new();

scene.show(&mol, &cv::select(&mol).atoms(), cv::BallAndStick::default())?;
scene.show(&mol, &oxygens, cv::styles::Sphere::default().scale(1.4))?;
scene.show(&mol, &aromatic, cv::styles::Stick::default().color(color))?;
```

`show` accepts a borrowed reusable selection. `cv::select(&source)` uses static trait dispatch implemented by the viewer for CK types; users need not import a trait or modify CK. Existing `mol.atoms()` and `mol.bonds()` remain CK data accessors. No conversion into a viewer-owned molecule is required.

Current protein rendering uses `ck::Protein`, `cv::select(&protein).residues()` with `cv::Cartoon`, and atom selections with `cv::Surface` or `cv::styles::Sphere`. `show_with_id` creates/replaces a named display atomically; `recenter_on(&source)` frames the source's all-atom geometric center without modifying scientific coordinates or depending on display IDs/styles. Molecules without stored coordinates require an explicit CK coordinate-generation operation. Multiple 3D conformers require an explicit `cv::styles::Coordinates::ThreeD { id }` choice for rendering; source-only recentering rejects that ambiguity.

Broader `BioStructure` and pocket selection remain a future extension of the same contract:

```rust
let structure = ck::BioStructure::from_mmcif(mmcif)?;
// A source-bound selection of ligand atoms belonging to this structure.
let ligand = ligand_selection;

let pocket = cv::select(&structure)
    .atoms()
    .protein()
    .within(6.0, &ligand)?
    .residues()
    .atoms();

scene.show(
    &structure,
    &cv::select(&structure).residues().protein(),
    cv::Cartoon::default(),
)?;
scene.show(&structure, &pocket, cv::styles::Stick::default())?;
scene.show(&structure, &ligand, cv::BallAndStick::default())?;
```

The independent selection layer evaluates the pocket query using CK coordinates and classifications. The renderer receives the resulting selected entities and derives geometry from the source.

## Python API

The current Python binding accepts CK objects directly. Molecular sphere/stick
styles use `SphereStyle`/`StickStyle` to preserve the positioned primitive names.

```python
import cosmolkit as ck
import cosmol_viewer as cv

mol = ck.Molecule.from_sdf(sdf)
oxygen = cv.select(mol).atoms().element(ck.Element.O)

scene = cv.Scene()
scene.show(mol, cv.select(mol).atoms(), style=cv.BallAndStick())
scene.show(mol, oxygen, style=cv.SphereStyle().scale(1.4))
```

The implemented call forms are:

```python
scene.show(source, selection, style=style)
scene.show(source, selection, style)
```

A future overload could omit `selection` and choose all source entities appropriate for the requested style, not an implicit chemical filter. The current binding requires an explicit selection.

Python accepts CK objects directly; viewer-owned chemical wrappers have been removed.
Both Python and JS check exact source identity and compare rendering inputs at
`show` time. Changed rendering data requires a fresh selection. This is an
O(source size) comparison, not a CK revision token: unrelated property changes
and edits that restore identical inputs are outside this check.

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
let oxygen = cv::select(&mol1).atoms().element(ck::Element::O);

// Must be rejected: oxygen belongs to mol1, not mol2.
scene.show(&mol2, &oxygen, cv::styles::Sphere::default())?;
```

Rejection should happen at compile time where the public types can enforce it. Otherwise, Rust must return a clear error and Python must raise a clear exception identifying the source mismatch. Matching entity counts, indices, or contents must not make an unrelated source acceptable.

Rust lifetimes can prevent a selection from outliving its borrowed source. A lifetime alone does not prove that two references point to the same source value; source identity must also be enforced.

The initial Rust API checks the exact borrowed source object. Equal contents and cloned CK values do not make another object acceptable, even if CK shares their underlying storage. A scene retains a shared CK snapshot after `show`; changing or dropping the original local variable does not retarget or invalidate that display.

Value-style topology changes create a new source value:

```rust
let oxygen = cv::select(&mol).atoms().element(ck::Element::O);
let hydrogenated = mol.with_hydrogens()?;

// Must be rejected even if some atom indices still match.
scene.show(&hydrogenated, &oxygen, cv::styles::Sphere::default())?;

// Recompute a selection against the new source.
scene.show(
    &hydrogenated,
    &cv::select(&hydrogenated).atoms().element(ck::Element::O),
    cv::styles::Sphere::default(),
)?;
```

The viewer must not silently remap, rebind, or reinterpret a selection against another topology value.

## Source lifetime and updates

- Rust displays must not outlive their source; the API must enforce valid borrowing or shared ownership.
- Python/JS selections retain their original source object. Completed displays and animation frames store geometry snapshots, so deleting a local source variable does not invalidate them.
- Adding a display does not mutate its source, topology, coordinates, properties, or selections.
- Replacing a local variable with a newly derived CK value does not retarget existing displays or selections.
- `show` does not establish an automatic source-mutation subscription. Live scientific updates require a separately specified explicit update contract.

## Future API boundaries

Trajectories, surfaces, and property coloring must preserve the same division: CK supplies scientific data; the independent selection layer supplies source-aware selections; the viewer supplies visual projections and display state.

This plan does not define implementation types, storage layouts, selection algorithms, transport protocols, display editing methods, or migration schedules. It does not change the existing `Viewer.render`, `Viewer.update`, or `Viewer.keep_alive` lifecycle contract.
