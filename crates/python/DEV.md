## Upgrade the Release Version

From the repository root, preview and then apply a release bump:

```bash
python tools/bump_version.py 0.5.0rc1 --dry-run
python tools/bump_version.py 0.5.0rc1
```

The script accepts `X.Y.Z`, Python-style `X.Y.ZrcN`, and Rust-style
`X.Y.Z-rc.N`. For this release, Python uses `0.5.0rc1` and Rust uses
`0.5.0-rc.1`.

It updates the workspace package version, the two versioned internal workspace
dependencies, and `crates/python/pyproject.toml`. The Rust facade, core,
Python binding, and WASM binding inherit the workspace version. It then runs
`cargo update --workspace` to refresh their lockfile entries without requesting
a blanket third-party dependency upgrade.

`--dry-run` only prints the planned edits; it does not write files or run Cargo.
All manifest edits are prepared before writing. If the Cargo command fails,
the script reports failure and leaves the version edits available for inspection
and retry.

COSMolKit dependency versions, the unpublished derive helper, experiments,
historical changelog entries, and the documentation site's published-package
pin are not release fields managed by this script. Generated WASM assets and
native binaries must be rebuilt through the normal build workflow; the script
does not publish packages, create tags, or commit changes.

Regression check:

```bash
python tools/test_bump_version.py -v
```

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
python crates/python/tests/test_viewer_backend.py
python crates/python/tests/test_native_viewer.py --gui
```

The second command opens native test windows and checks child process cleanup.

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
