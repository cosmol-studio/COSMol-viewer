# Python development

All commands below run from the repository root. Development tooling requires
Python 3.11+, Rust, wasm-pack and a working native compiler/linker. Windows
requires Visual Studio C++ Build Tools and a Windows SDK. The published Python
package supports Python 3.9+; that is separate from the tooling requirement.

## Prepare the Python environment

Create a virtual environment, or use an existing environment of your choice:

```sh
python -m venv .venv
```

Activate it in Ubuntu Bash:

```sh
source .venv/bin/activate
```

Or in Windows PowerShell:

```powershell
.venv/Scripts/Activate.ps1
```

Install the build tools (`maturin`, `uv`), type checker (`basedpyright`), and
the published COSMolKit runtime dependency (`cosmolkit`) into that environment.
This does not install COSMol Viewer; the next section compiles and installs it.

```sh
python -m pip install "maturin==1.15.0" uv basedpyright "cosmolkit==0.6.2"
```

## Build and install for development

First generate the browser assets embedded by the Python Notebook backend:

```sh
rustup target add wasm32-unknown-unknown
wasm-pack build wasm --target web --dev --no-opt --out-dir pkg --out-name cosmol_viewer_wasm --locked
```

This produces `wasm/pkg/cosmol_viewer_wasm.js` and
`wasm/pkg/cosmol_viewer_wasm_bg.wasm`. Python embeds these files during compilation;
its build does not invoke wasm-pack. npm package preparation is not required for
Python or Notebook use. See [WASM development](../wasm/DEV.md) for release flags.

Next build the native renderer, then compile and install the Python extension:

The extension install includes type declaration generation:
`maturin develop --generate-stubs` compiles the extension, extracts `.pyi`
declarations from the compiled binary, then installs both into the active Python
environment. You do not need to generate `python/cosmol_viewer.pyi` first.

```sh
python python/build_native_viewer.py
maturin develop --uv --locked --generate-stubs --manifest-path python/Cargo.toml
python -c "import cosmol_viewer as cv; print(cv.__file__); print(cv.Scene())"
```

`build_native_viewer.py` builds `cosmol-viewer-native` and copies it into
`python/cosmol_viewer_native/` for packaging. `maturin develop` builds the PyO3
extension, generates type declarations from that binary and installs the package
into the active Python environment. Direct maturin commands do not invoke
`build_backend.py`, so the native-renderer command must run first.

After Python binding changes, rerun `maturin develop`. After core/renderer changes,
rerun both commands. After Notebook/WASM changes, rebuild WASM before compiling
the extension so it embeds the new assets.

## Update repository stubs and run contributor checks

The development install above is complete, including its installed type
declarations. This section is a separate contributor workflow for updating the
tracked `python/cosmol_viewer.pyi` and checking binding changes; it is not a
prerequisite for `maturin develop` and does not have to run after it either.

Both workflows generate declarations from a compiled extension. Their purposes
and destinations differ:

| Command | Purpose | Type declaration destination |
| --- | --- | --- |
| `maturin develop --generate-stubs` | Build and install for local use | Installed Python package |
| `python python/build.py --install-extractor --check` | Update tracked declarations and test the bindings | `python/cosmol_viewer.pyi`, also staged with the extension under `target/python-bindings/package/` for tests |

After changing Rust/PyO3 declarations or typed converters, run the following to
build the checkout extension, regenerate the tracked `.pyi`, and run runtime and
type checks. The script builds the extension itself before extracting its types:

```sh
python python/build.py --install-extractor --check
```

`--install-extractor` installs the official pyo3-introspection version matching
PyO3 into `target/pyo3-tools/`. On subsequent runs it can be omitted:

```sh
python python/build.py --check
```

This stages the extension and stub under `target/python-bindings/package/` for
the tests. It does not install the package into the active environment or build
the native renderer. Use `maturin develop` above to update your installed package.
The generated `.pyi` must not be edited manually.

Native window lifecycle tests are opt-in. After the development install, run:

```sh
python python/tests/test_native_viewer.py --gui
```

## Build a release wheel

Build the release WASM assets first using [WASM development](../wasm/DEV.md#build-wasm),
then build the native executable and wheel with the release profile:

```sh
python python/build_native_viewer.py --release
maturin build --release --locked --generate-stubs --manifest-path python/Cargo.toml --out dist
```

The wheel is written to `dist/` and contains the extension, generated declarations
and native renderer. The Notebook assets are embedded in the extension.

## Install through pip

For an editable install that invokes the package build backend automatically:

```sh
python -m pip install --editable ./python
```

After the WASM build, this is an alternative to the two explicit development
build commands. `python/build_backend.py` builds the native renderer and delegates
to maturin with stub generation enabled. pip installs the pinned maturin version
in its isolated build environment. Rerun the install after Rust or embedded WASM
changes; an editable install does not recompile Rust automatically.
