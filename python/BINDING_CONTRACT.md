# Python binding contract

The Rust implementation owns behavior. PyO3 declarations own Python names,
signatures, defaults, documentation and type conversion. The official
`experimental-inspect` / `pyo3-introspection` pipeline extracts those declarations
from the compiled extension. The workspace pins the PyO3 version; the setup
command installs that same extractor version.

## Ownership

- `src/lib.rs` declares exports with an inline `#[pymodule]`. Do not replace it
  with dynamic registration that hides exports from official introspection.
- `src/scene.rs`, `animation.rs`, `styles.rs` and `shapes/` project native values. Scene
  insertion and animation frames retain snapshots; shape fluent methods return
  the same Python shape. Indexing an animation returns an independent scene.
- `src/image.rs` owns Python image-export adaptation and its subprocess
  lifecycle. `viewer.rs` and `notebook.rs` own host selection and interactive
  display. Rendering and chemistry remain in their existing Rust owners.
- `framework/` owns generic extraction/build and typed foreign-class references.
  It must not import viewer business types, enumerate API names, translate
  chemistry, or patch individual signatures. New business APIs do not extend
  the framework or its fixed consumer samples.

## Types come from actual conversions

Use concrete Rust signatures and `#[derive(FromPyObject)]` unions for ordinary
input alternatives. The same variant declaration drives accepted runtime types
and official annotations. Shared styling is a native method projection, not a
second list of Python signatures.

For a custom converter, derive its type information from the constituent Rust
converters or declare its foreign class once through `ExternalType`. The generic
`External<T>` adapter uses that declaration for both `isinstance` validation and
`FromPyObject::INPUT_TYPE`; business transport stays in the declaring module.

Do not add handwritten `.pyi` fragments, Python signature strings, post-extraction
substitutions, or checker-local exclusions to make an API appear typed. Generated
declarations containing `Any` or `Incomplete` fail the development build. Official
array annotations describe Python sequences; fixed lengths and numeric ranges
remain runtime checks.

## Input and error behavior

Complete conversion before modifying native state. Invalid input types raise
`TypeError`; invalid color values raise `ValueError`; missing named shapes raise
`KeyError`; invalid animation indices raise `IndexError`. Native operation errors
retain the existing binding error policy. Exceptions from foreign Python calls
propagate with their identity; do not replace them with guessed fallback values.

Color strings accept six ASCII hex digits, optionally prefixed by `#`. Image
backgrounds additionally accept eight-digit RGBA hex, RGB/RGBA byte sequences,
normalized float RGBA and the documented background selectors. Failed color
conversion does not change the shape or scene. Image paths accept `str` and
`os.PathLike[str]`, passed literally without `~` expansion.

The selection adapter accepts actual `cosmolkit.Molecule` and `cosmolkit.Protein`
objects. The binding imports the source once into this extension's native CK
value: both Molecule and Protein use `to_transfer_binary()` and the corresponding
native `ck::Molecule::from_transfer_binary` / `ck::Protein::from_transfer_binary`.
Rust pins CK `0.6.2` and Python pins `0.6.2` for the transfer format's
exact-version requirement. Protein is decoded directly without mmCIF conversion
or another protein projection.

All selection IDs come from the imported native source. `select`, atom/residue/
bond selection, `element` and `aromatic` delegate to core Rust selection methods.
The binding retains native IDs and reborrows a core selection from that same
native source when calling Rust. It does not traverse foreign scientific fields,
compare rendering snapshots, infer mutations, or map original Protein IDs.
`Scene.add`, `set` and `recenter_on` call the corresponding core Scene
methods with the native CK object. Core Rust owns validation, geometry generation,
and atomic insertion/replacement. Python only converts arguments, dispatches concrete types and retains their
lifetimes. `add` and `set` accept positioned shapes, entire CK objects or selections; selections carry
their source and never require a second source argument. Core Rust chooses the
whole-object entity set from the style. `Viewer.show` creates a viewer.

Live selectors, selections and Scenes share an immutable import by Python object
identity. A weak registry does not keep discarded imports alive; each live import
retains its Python owner to prevent address reuse. Source edits do not update an
existing import. To import edited data while the old snapshot is still in use,
pass a new CK object (for example a copy). Once all owners of an import are gone,
a later use imports the object again. No per-platform mutation heuristics run.
Cloned Scenes share imports; animation frames retain core geometry snapshots.
Completed geometry survives deletion of the original Python variables.

`BallAndStick`, `SphereStyle`, `StickStyle`, `Cartoon` and `Surface` configure
scientific projections. `Sphere` and `Stick` remain positioned primitives.
No viewer-defined `Molecule` or `Protein` class is exported.

## Validation and packaging

`python python/build.py --check` compiles, extracts verbatim official
stubs, checks all exported runtime methods against declarations, runs behavioral
regressions and checks typed consumers. The contract registry remains a semantic
cross-language audit; it does not manufacture Python signatures or types.

Wheel and editable builds use `maturin --generate-stubs`, extracting from the
binary being packaged. PEP 517 enables this automatically. Generated files are
outputs, never additional API sources. See [DEV.md](DEV.md) for commands.

Styles are optional. The core owns all defaults: Molecule uses BallAndStick and
Protein uses Cartoon. Atom selections use Sphere regardless of their source,
bond selections use Stick, and residue selections use Cartoon. Positioned shapes retain their material and reject
an additional scientific style. Python passes the optional style through without
choosing a platform-specific default. The old shape-specific insertion/replacement
methods are removed; `set` creates or replaces named content of any supported kind.
