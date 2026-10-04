# Python Binding Development

Project-wide release version upgrades are documented in the
[root development guide](../../DEV.md#upgrade-the-release-version).

## Generate `.pyi` Stubs (dev / abi3-py310)

From repo root:

```bash
cargo run -p cosmol_viewer_python --no-default-features --features dev-stub --bin stub_gen
```

Generated file:

```text
./crates/python/cosmol_viewer.pyi
```

## Build/Install Extension with maturin

### Dev install (editable)

From repo root:

```bash
python crates/python/build_native_viewer.py
maturin develop --uv --manifest-path crates/python/Cargo.toml
```

The first command builds the standalone renderer and stages it in
`crates/python/cosmol_viewer_native/`. Re-run it after changing native Rust code.
The renderer is included in wheels; Python never restarts `python.exe --child`.

For a release wheel using the maturin CLI:

```bash
python crates/python/build_native_viewer.py --release
maturin build --release --manifest-path crates/python/Cargo.toml
```

PEP 517 builds (`pip install`, `uv pip install`) use `build_backend.py` to build
the renderer automatically before delegating to maturin. Cross builds should
pass the same target to the renderer and the extension (`--target` or
`CARGO_BUILD_TARGET`). `COSMOL_VIEWER_NATIVE_PATH` can override the bundled
renderer path during development.

Regression checks:

```bash
python crates/wasm/tools/wasm_binding/build.py --check
python crates/python/tests/test_binding_surface.py
python crates/python/tests/test_viewer_backend.py
python crates/python/tests/test_native_viewer.py --gui
```

Install Alef 0.103.12 and wasm-pack before the first command. It regenerates the
browser package, runs actual WASM exports in Node, and checks generated TypeScript
types. Add `--browser-check` for the real WebGL rendering smoke test (requires
Playwright and Chromium). See `crates/wasm/tools/wasm_binding/README.md` for setup
and the typed `Viewer` / `Scene` / `Animation` JavaScript API.

The Notebook command test mocks notebook display and verifies captured payloads
against the real WASM receiver's pre-initialization errors. It does not test WebGL
rendering or successful frame updates. The last command opens native test windows
and checks child process cleanup.

## Build Python Documentation

From the repository root:

```bash
uv venv .venv
uv pip install --python .venv/bin/python "maturin>=1.7,<2.0" "sphinx>=8,<10" "furo>=2024.8.6"
.venv/bin/maturin develop --manifest-path crates/python/Cargo.toml
.venv/bin/python -m sphinx -W --keep-going -b html crates/python/docs/source crates/python/docs/_build/html
```

Generated HTML:

```text
crates/python/docs/_build/html/index.html
```

On Windows, run the equivalent commands after ``conda activate COS`` and use
``maturin`` and ``python`` from that environment.
