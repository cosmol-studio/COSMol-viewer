# Python binding contract

The Rust implementation owns behavior. PyO3 declarations own Python names,
signatures, defaults, documentation and type conversion. The official
`experimental-inspect` / `pyo3-introspection` pipeline extracts those declarations
from the compiled extension. The workspace pins the PyO3 version; the setup
command installs that same extractor version.

## Ownership

- `src/lib.rs` declares exports with an inline `#[pymodule]`. Do not replace it
  with dynamic registration that hides exports from official introspection.
- `src/scene.rs`, `animation.rs` and `shapes/` project native values. Scene
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

The COSMolKit adapter accepts actual `cosmolkit.Molecule` objects. It reads the
canonical atom/bond/coordinate members, supporting their property and previous
zero-argument accessor representations. It generates 2D coordinates only when
no coordinate block exists, never after an arbitrary failure. Coordinate row
counts and dimensions must match exactly, and values must fit finite `f32`.

## Validation and packaging

`python python/build.py --check` compiles, extracts verbatim official
stubs, checks all exported runtime methods against declarations, runs behavioral
regressions and checks typed consumers. The contract registry remains a semantic
cross-language audit; it does not manufacture Python signatures or types.

Wheel and editable builds use `maturin --generate-stubs`, extracting from the
binary being packaged. PEP 517 enables this automatically. Generated files are
outputs, never additional API sources. See [DEV.md](DEV.md) for commands.
