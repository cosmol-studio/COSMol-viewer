# WASM development

Run from the repository root. Install Rust, wasm-pack and Node/npm.
Python 3.11+ is needed only for the automation and test scripts.

## Build WASM

```sh
rustup target add wasm32-unknown-unknown
wasm-pack build wasm --target web --release --out-dir pkg --out-name cosmol_viewer_wasm --locked
```

This generates `wasm/pkg/cosmol_viewer_wasm_bg.wasm`, the JavaScript loader,
TypeScript declarations and `package.json`. wasm-pack invokes the official
wasm-bindgen matching the Rust dependency. Do not hand-edit generated JS or types.
For a development build, replace `--release` with `--dev --no-opt`.

## Configure package.json

This section is for npm/JavaScript consumers. Python Notebook needs only the
two generated files from the build above; it needs no npm setup or CK JS package.

Copy `wasm/js/index.js` to `wasm/pkg/index.js` (PowerShell:
`Copy-Item wasm/js/index.js wasm/pkg/index.js`; bash:
`cp wasm/js/index.js wasm/pkg/index.js`). Then run:

```sh
cd wasm/pkg
npm pkg set "name=@cosmol-studio/cosmol-viewer" "type=module" "main=index.js" "dependencies.@cosmol-studio/cosmolkit=0.6.2-rc.1"
node --input-type=module -e "import fs from 'node:fs'; const p=JSON.parse(fs.readFileSync('package.json','utf8')); p.files=[...new Set([...p.files,'index.js'])]; fs.writeFileSync('package.json',JSON.stringify(p,null,2)+'\n');"
cd ../..
```

The resulting fields are:

```json
{
  "name": "@cosmol-studio/cosmol-viewer",
  "type": "module",
  "main": "index.js",
  "dependencies": {
    "@cosmol-studio/cosmolkit": "0.6.2-rc.1"
  }
}
```

Keep the generated version and type declarations; add `index.js` to `files`.
The version comes from Cargo. The dependency above matches
`package.metadata.javascript.dependencies` in `wasm/Cargo.toml`.
This metadata is consumed by our automation; the manual command above sets it
explicitly. The npm entry point imports CK's existing classes and registers them
after Viewer initialization, so the JS object adapters can perform instance checks.
CK and CV remain separate packages; no JS bundling is performed.

Notebook embeds `cosmol_viewer_wasm.js` and `cosmol_viewer_wasm_bg.wasm` directly.
The generated loader has no runtime CK import. Notebook sends serialized CV
scenes/commands, not CK JS objects, and does not use the npm entry point.

For automation, `python wasm/build.py --release` runs wasm-pack and prepares npm
metadata and the entry point. It does not install CK, bundle JS or run tests.
Without `--release` it builds in development mode.

## WASM tests (optional)

These are independent of publishing. Build and configure `wasm/pkg` above first.

| Files | What they test |
| --- | --- |
| `tests/converters/` | Typed input/output conversion, runtime behavior, TypeScript types and expected Rust compile errors |
| `tests/binding_surface.mjs` | Generated JavaScript API behavior in Node |
| `tests/binding_surface.ts` | TypeScript consumer types |
| `tests/browser_render.mjs` | Actual Chromium rendering |
| `tests/binding_contract.rs`, `tests/scene.rs` | Native Rust binding contracts and scene behavior |

Run the converter, Node and TypeScript tests against the existing package:

```sh
npm install --prefix wasm/pkg --ignore-scripts --no-audit --no-fund --package-lock=false
python wasm/check.py
```

This does not rebuild the viewer WASM. It builds a separate converter test fixture
under `target/wasm-check/converters`, which is not distributed. The script uses
an installed TypeScript compiler or downloads TypeScript 5.8.3 via npx.

For Chromium rendering, install Playwright and its browser first:

```sh
npm install --prefix target/browser-check --no-save playwright@1.58.2
target/browser-check/node_modules/.bin/playwright install chromium
python wasm/check.py --browser
```

On Windows, use `target/browser-check/node_modules/.bin/playwright.cmd`.
On Ubuntu 24.04, use `install --with-deps chromium` to install system dependencies.
Playwright 1.58.2 cannot install Chromium on Ubuntu 26.04.
`PLAYWRIGHT_MODULE` and `BROWSER_EXECUTABLE` select alternative installations.

Run the native Rust tests separately:

```sh
cargo test -p cosmol_viewer_wasm --locked --no-default-features --test binding_contract --test scene
```
