# Documentation

Documentation needs generated WASM bindings and CK types, not npm release packaging.
Run the commands below from the repository root; see [WASM development](../wasm/DEV.md)
for tool installation.

The shared Sphinx project lives here, with Python pages in `source/python/`,
JavaScript / TypeScript pages in `source/javascript/`, and common styles,
templates and API extensions shared by both languages. Generated HTML belongs
in `build/html/`.

From the repository root, prepare the bindings and documentation tools:

```sh
uv sync --group dev
cargo install pyo3-introspection --version 0.30.1 --locked
npm ci --prefix docs --no-audit --no-fund
wasm-pack build wasm --target web --dev --no-opt --out-dir pkg --out-name cosmol_viewer_wasm --locked
npm install --prefix wasm/pkg --no-save --ignore-scripts --no-audit --no-fund --package-lock=false @cosmol-studio/cosmolkit@0.6.2-rc.1
.venv/bin/python python/build.py
```

Reuse the built bindings and declarations for documentation-only edits. Build
both language guides and API references together:

```sh
.venv/bin/python -m sphinx -W --keep-going -E -b html docs/source docs/build/html
```

Python API signatures come from the native extension and generated stub;
JavaScript API signatures come from the public TypeScript declarations via
TypeDoc and sphinx-js. Do not maintain handwritten signature inventories.

The [Dioxus site](../docs-web/README.md) consumes this output and preserves
the public `/python`, `/python/api`, `/javascript` and `/javascript/api` routes.
Its route contract is `docs-web/routes.toml`; source paths are not public URLs.

The shared implementation and Viewer-specific adaptations are recorded in
[CK parity](CK_PARITY.md).

On Windows, use the checked-in Sphinx runner after installing the same tools:

```powershell
.venv/Scripts/python.exe docs/sphinx_build.py -W --keep-going -E -b html docs/source docs/build/html
.venv/Scripts/python.exe -m unittest discover -s tools -p test_python_docs.py -v
```

This runner handles sphinx-js 5 process paths and temporary-file sharing on
Windows. On Linux it delegates directly to Sphinx. It never changes API
signatures or generated pages. Dioxus's automatic Sphinx build uses this runner
as well; no files under `target/` are required to invoke it.

## Shared guides

Quick Start is the first shared guide. Its prose and paired examples live in
`source/shared/quickstart.inc`; the Python and JavaScript `.rst` entry points
only supply their own metadata and include that source.

`code-example` takes a stable example ID and exactly one `python` and one
`javascript` code block. `language-only` contains notes for capabilities that
differ. The `code_examples` extension emits only the URL-selected language and
links each example to the matching section of the other language route.
Switching any example updates the entire page, with native history and reload
behavior, including when JavaScript is disabled. Keep matching routes paired by
`topic` in `docs-web/routes.toml` when extending this to other guides.
