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

The molecular example uses the current Rust API. `add` is fallible so invalid source or style combinations can be reported explicitly. The initial selection API supports molecular atoms/bonds and protein atoms/residues, element filtering and aromatic bond filtering. It does not yet implement BIO pocket queries or protein chemical-bond projections.

```rust
let mol = ck::Molecule::from_sdf(sdf)?;

let oxygens = cv::select(&mol).atoms().element(ck::Element::O);
let aromatic = cv::select(&mol).bonds().aromatic();

let mut scene = cv::Scene::new();

scene.add((&mol, cv::BallAndStick::default()))?;
scene.add((&oxygens, cv::styles::Sphere::default().scale(1.4)))?;
scene.add((&aromatic, cv::styles::Stick::default().color(color)))?;
```

`add` accepts an entire CK object or a borrowed reusable selection; `set(name, (item, style))` creates or replaces named content. Selections already carry their source, so callers never pass it again. `cv::select(&source)` uses static trait dispatch implemented by the viewer for CK types; users need not import a trait or modify CK. Existing `mol.atoms()` and `mol.bonds()` remain CK data accessors. No conversion into a viewer-owned molecule is required.

Whole proteins can be passed directly with `cv::Cartoon`, `cv::Surface` or `cv::styles::Sphere`. The Rust core selects all residues for Cartoon and all atoms for other styles. Explicit selections remain available for filtered displays. `set` creates/replaces a named display atomically; `recenter_on(&source)` frames the source's all-atom geometric center without modifying scientific coordinates or depending on display IDs/styles. Molecules without stored coordinates require an explicit CK coordinate-generation operation. Multiple 3D conformers require an explicit `cv::styles::Coordinates::ThreeD { id }` choice for rendering; source-only recentering rejects that ambiguity.

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

scene.add((&cv::select(&structure).residues().protein(), cv::Cartoon::default()))?;
scene.add((&pocket, cv::styles::Stick::default()))?;
scene.add((&ligand, cv::BallAndStick::default()))?;
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
scene.add(mol, style=cv.BallAndStick())
scene.add(oxygen, style=cv.SphereStyle().scale(1.4))
```

The implemented call forms are:

```python
scene.add(mol, cv.BallAndStick())
scene.set("protein", protein, cv.Cartoon())
scene.add(protein, cv.Surface())
scene.add(oxygen, cv.SphereStyle())
viewer = cv.Viewer.show(scene)
```

Python and JavaScript import each live CK object through its transfer binary once.
Scenes, selectors and selections share that native snapshot and retain its foreign
owner. There is no field traversal or repeated comparison of scientific data.
Pass a new CK object to import edited data while the earlier snapshot remains live.
The cache uses weak entries, so it does not keep unused imports alive.

## Display behavior

Each successful `add` call adds an independent display associating the source, selection, and style. It does not replace earlier displays of the same source.

- One CK object can contribute any number of displays without duplicating its scientific structure for each display.
- Overlapping selections are allowed. Styling one display does not change the CK object or another display.
- A protein can appear simultaneously as a cartoon, pocket sticks, and ligand ball-and-stick geometry.
- Displays belong to the scene. CK objects do not store viewer representations or render styles.
- Styles consume compatible entity kinds: for example, spheres use atoms, sticks can use bonds, and cartoons use suitable residues and connectivity. Unsupported combinations produce an explicit error rather than silently changing the selection.
- An empty compatible selection produces no geometry and is not an error.
- A rejected `add` call adds no display and leaves existing scene state unchanged.

Keeping several displays does not imply a second editable scientific model. Derived geometry, GPU buffers, and transport payloads are permitted; they are visual data, not an independently mutable molecule or protein API. This contract does not promise zero-copy GPU upload or IPC.

## Selection identity and validity

Every selection carries its CK source. `add(selection, style)` and
`set(name, selection, style)` obtain the source from that selection; there is no
separate source parameter that could accidentally rebind its indices.

```rust
let oxygen = cv::select(&mol).atoms().element(ck::Element::O);
scene.add((&oxygen, cv::styles::Sphere::default()))?;
let hydrogenated = mol.with_hydrogens()?;
// Select from a new value explicitly when its topology should be used.
let new_oxygen = cv::select(&hydrogenated).atoms().element(ck::Element::O);
scene.set("oxygen", (&new_oxygen, cv::styles::Sphere::default()))?;
```

Rust selections borrow their source; Python and JavaScript selections retain the
shared native import. Displays retain CK snapshots and derived geometry. The
viewer never silently remaps a selection onto another topology.

## Source lifetime and updates

- Rust displays must not outlive their source; the API must enforce valid borrowing or shared ownership.
- Python/JS selections retain their original source object. Completed displays and animation frames store geometry snapshots, so deleting a local source variable does not invalidate them.
- Adding a display does not mutate its source, topology, coordinates, properties, or selections.
- Replacing a local variable with a newly derived CK value does not retarget existing displays or selections.
- `add` does not establish an automatic source-mutation subscription. Live scientific updates require a separately specified explicit update contract.

## Future API boundaries

Trajectories, surfaces, and property coloring must preserve the same division: CK supplies scientific data; the independent selection layer supplies source-aware selections; the viewer supplies visual projections and display state.

This plan does not define implementation types, storage layouts, selection algorithms, transport protocols, display editing methods, or migration schedules. It does not change the existing `Viewer.show`, `Viewer.update`, or `Viewer.keep_alive` lifecycle contract.

## Unified content and default styles

Rust accepts `scene.add(item)` and `scene.set(name, item)` for positioned shapes,
CK objects and selections. An explicit style is paired with the object as a tuple:
`scene.add((&protein, Surface::default()))`. Python/JS use a separate optional
style parameter. The core defaults are BallAndStick for Molecule, Cartoon for
Protein/residue selections, Sphere for atom selections regardless of their source,
and Stick for bond selections. Shapes carry their own material. Named content can be replaced across
kinds, and invalid projections leave the earlier content unchanged.
