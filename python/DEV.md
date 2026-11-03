# Build and install the Python package

Run from the repository root with Rust installed and Python 3.11+.
On Windows, activate `conda activate COS`.

First complete the [WASM build](../wasm/DEV.md#build-wasm).
The Python build embeds the existing `wasm/pkg/cosmol_viewer_wasm.js` and
`wasm/pkg/cosmol_viewer_wasm_bg.wasm`; it does not build WASM.
Notebook receives serialized CV scenes and commands, without CK JavaScript.

Build and install into the current Python environment:

```sh
python -m pip install --editable ./python "cosmolkit==0.6.2rc1"
```

This invokes `python/build_backend.py`, which builds the native renderer and
uses maturin to compile the Python extension and generate its `.pyi` declarations.
The build environment installs the pinned `maturin==1.15.0` automatically.
No separate stub-generation or native-renderer command is required.
Rerun this command after Rust changes, or after rebuilding the WASM assets.
Generated `.pyi` files must not be edited manually.
