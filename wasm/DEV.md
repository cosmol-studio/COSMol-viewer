# WASM development

Run from the repository root. Install Rust's
`wasm32-unknown-unknown` target, wasm-pack, Node and Python 3.11+.

```sh
python wasm/build.py --check
python wasm/build.py --release --check --browser-check
```

The first command builds the crate directly, extracts official JS/TypeScript,
runs Node regressions and strict TypeScript checks, then stages `wasm/pkg`.
Checks include a fixed converter fixture for Tsify/Serde scalar and structured
input/output behavior; that separate package is not distributed.
The second enables release optimization and the real Chromium rendering tests.
`--no-opt` skips wasm-opt; `WASM_PACK_BIN`, `NODE_BIN`, `NPX_BIN` select tools.
The manifest pins wasm-bindgen; wasm-pack installs the matching CLI for extraction.
The build installs the external dependencies declared in Cargo package metadata
and bundles a self-contained `notebook.js` with esbuild for Blob-module loading.
Ordinary JS/TypeScript declarations remain official wasm-bindgen outputs.

Browser checks need Playwright:

```sh
npm install --prefix target/browser-check --no-save playwright@1.58.2
target/browser-check/node_modules/.bin/playwright install chromium
```

Linux CI uses `install --with-deps chromium`. `PLAYWRIGHT_MODULE` and
`BROWSER_EXECUTABLE` select alternative installations.

Native model/transport checks:

```sh
cargo test -p cosmol_viewer_wasm --no-default-features
```

Build WASM before Python to embed the current notebook receiver. After changing
the browser package, `python python/build.py --check` checks Python notebook
payloads against that receiver. Generated package files are not source files.
