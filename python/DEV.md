# Python Binding Development

Run these commands from the repository root. On Windows, activate `conda activate COS`.
Project release version changes are described in [the root guide](../DEV.md).

## Build and extract official types

```bash
python -m pip install "maturin==1.15.0" "basedpyright==1.40.2" cosmolkit
python wasm/build.py --check
python python/build.py --install-extractor --check
```

`--install-extractor` installs `pyo3-introspection` with the exact PyO3 version
from the workspace Cargo manifest. It is only needed for initial setup or a
PyO3 version change. Subsequent builds use:

```bash
python python/build.py --check
```

This compiles the extension, extracts the official `.pyi`, stages an importable
test extension in `target/python-bindings/package`, runs Python regressions and
checks consumer types with BasedPyright. Native GUI tests remain opt-in. Notebook
tests capture Python payloads and check them against the existing WASM receiver;
they do not claim real notebook rendering coverage.

`python/cosmol_viewer.pyi` is generated output. Do not edit it. Change the
Rust/PyO3 declaration or input/output conversion, then rerun the build. The
extractor output is copied verbatim: no handwritten signature inventory, stub
injection, or function-specific postprocessing is maintained. Missing types fail
the build. `--extractor PATH` or `PYO3_INTROSPECTION` selects an explicit matching
extractor; `--release` requests an optimized extension.

## Install and package

```bash
python python/build_native_viewer.py
maturin develop --uv --generate-stubs --manifest-path python/Cargo.toml
```

The first command stages the standalone renderer in `cosmol_viewer_native/`.
Rebuild it after native Rust changes. `--generate-stubs` extracts declarations
from the exact extension being installed, so the install cannot reuse a stale
local `.pyi`.

For release wheels:

```bash
python python/build_native_viewer.py --release
maturin build --release --generate-stubs --manifest-path python/Cargo.toml
```

PEP 517 builds (`pip install`, `uv pip install`) use `build_backend.py` to build
the renderer and enable official stub generation automatically. Cross builds
must use the same target for renderer and extension (`--target` or
`CARGO_BUILD_TARGET`). `COSMOL_VIEWER_NATIVE_PATH` overrides the renderer path
during development. Wheels retain ABI3 Python 3.9 support; build/check tooling
uses Python 3.11+ for `tomllib`.

## Architecture and extending bindings

See [BINDING_CONTRACT.md](BINDING_CONTRACT.md). PyO3 is the declaration and
conversion framework; `framework/` contains only generic build/extraction and
external-class reference mechanisms. Scene, animation, image export, host
backends and individual shape projections live in separate Rust modules.

To add an ordinary API, implement its native forwarding method with concrete
Rust types, defaults and documentation, then export it through the declarative
PyO3 module. `FromPyObject` enums derive union inputs and their annotations from
the same variants. The build tool and framework tests do not enumerate business
methods and must not need edits for each new API.

## Additional validation

```bash
cargo test -p cosmol_viewer_python --no-default-features --lib
python python/tests/test_native_viewer.py --gui
```

The GUI test opens native windows and verifies process cleanup. Browser package
changes use the independent [WASM workflow](../wasm/DEV.md).
Python bindings do not require changing that generation pipeline.

## Build documentation

After installing the extension:

```bash
python -m pip install "sphinx>=8,<10" "furo>=2024.8.6" sphinx-sitemap
python -m sphinx -W --keep-going -b html python/docs/source python/docs/_build/html
```
