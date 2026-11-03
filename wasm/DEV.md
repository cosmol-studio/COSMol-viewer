# WASM development

Run from the repository root. The same commands work in Ubuntu Bash and Windows
PowerShell. Install Rust, wasm-pack, Node/npm and Python 3.11+.
The `python` command must select Python 3.11+; activate a virtual environment
on Ubuntu or `conda activate COS` on the Windows development machine.

## Build WASM

Install the compilation target once:

```sh
rustup target add wasm32-unknown-unknown
```

Development build:

```sh
wasm-pack build wasm --target web --dev --no-opt --out-dir pkg --out-name cosmol_viewer_wasm --locked
```

For a release build, use this command instead:

```sh
wasm-pack build wasm --target web --release --out-dir pkg --out-name cosmol_viewer_wasm --locked
```

Output goes directly to **`wasm/pkg`**. wasm-pack resolves `--out-dir pkg`
relative to the `wasm` crate and creates the directory automatically.
Cargo reuses the workspace's `target` cache, or an existing `CARGO_TARGET_DIR`.
Stop if a command fails.

wasm-pack runs Cargo and the matching official wasm-bindgen. The outputs are:

- `cosmol_viewer_wasm_bg.wasm`: compiled WebAssembly.
- `cosmol_viewer_wasm.js`: browser loader and JavaScript bindings.
- `cosmol_viewer_wasm.d.ts`: TypeScript declarations, including tsify types.
- `package.json`: generated metadata, including version, license and repository.

Do not edit generated JS, declarations or WASM. `--locked` is passed to Cargo.
Release builds run wasm-opt with the options in
`wasm/Cargo.toml`, under `package.metadata.wasm-pack.profile.release`.
Add `--no-opt` to the release command to skip optimization.
The same manifest configures wasm-bindgen's glue/debug output.

## Prepare the npm package

After each wasm-pack build, copy the entry point and documentation:

```sh
cp wasm/js/* wasm/pkg/
cp wasm/README.md wasm/pkg/
cp LICENSE wasm/pkg/
```

Set the npm-specific metadata. Other fields, including the Viewer version,
description, license and repository, remain as generated
from Cargo:

```sh
npm --prefix wasm/pkg pkg set 'name=@cosmol-studio/cosmol-viewer'
npm --prefix wasm/pkg pkg set 'type=module'
npm --prefix wasm/pkg pkg set 'main=index.js'
npm --prefix wasm/pkg pkg set 'types=index.d.ts'
npm --prefix wasm/pkg pkg set 'sideEffects[]=./index.js'
npm --prefix wasm/pkg pkg set 'dependencies.@cosmol-studio/cosmolkit=0.6.2-rc.1'
```

The CK dependency must match
`package.metadata.javascript.dependencies` in `wasm/Cargo.toml`.
CK and Viewer are separate packages. The copied entry point registers CK's
classes after automatically initializing both WASM modules; it does not bundle
CK into Viewer. The `sideEffects` entry preserves this initialization when a
bundler removes unused exports. `index.d.ts` re-exports generated types without
advertising a default initializer.

Append the JS and type entry points to wasm-pack's generated file list. npm automatically
includes README and LICENSE:

```sh
npm --prefix wasm/pkg pkg set 'files[]=index.js'
npm --prefix wasm/pkg pkg set 'files[]=index.d.ts'
```

Run this once after each build; repeating the append without rebuilding adds
duplicate entries.

Set public access:

```sh
npm --prefix wasm/pkg pkg set 'publishConfig.access=public'
```

Choose **one** command matching the package version:

```sh
npm --prefix wasm/pkg pkg set 'publishConfig.tag=rc'      # X.Y.Z-rc.N (current)
npm --prefix wasm/pkg pkg set 'publishConfig.tag=dev'     # X.Y.Z-dev.N
npm --prefix wasm/pkg pkg set 'publishConfig.tag=latest'  # X.Y.Z
```

Inspect the package without uploading it:

```sh
npm pack --dry-run --json ./wasm/pkg
```

The file list should contain the generated bindings, `index.js`, README and
LICENSE, without `node_modules`.

## Python Notebook assets

Notebook embeds `wasm/pkg/cosmol_viewer_wasm.js` and
`wasm/pkg/cosmol_viewer_wasm_bg.wasm` directly. The wasm-pack build alone
produces these files; Notebook does not need npm metadata or installed CK JS.
It uses serialized Viewer scenes and commands, not the npm entry point.

## WASM checks

Prepare the npm package above, then install its dependencies and check it:

```sh
npm install --prefix wasm/pkg --ignore-scripts --no-audit --no-fund --package-lock=false
python wasm/check.py
```

This checks the existing Viewer package with Node and TypeScript and builds
a separate converter test fixture under `target/wasm-check/converters`.
It does not rebuild Viewer. TypeScript 5.8.3 is downloaded with npx if no
compiler is installed.

For browser rendering:

```sh
npm install --prefix target/browser-check --no-save playwright@1.58.2
node target/browser-check/node_modules/playwright/cli.js install --with-deps chromium
python wasm/check.py --browser
```

Calling the Playwright CLI through Node works in both shells. Linux browser
system dependencies can require administrator access. This Playwright version
targets Ubuntu 24.04; Ubuntu 26.04 requires a compatible browser installation.
`PLAYWRIGHT_MODULE` and `BROWSER_EXECUTABLE` can select existing installations.

Native Rust binding tests:

```sh
cargo test -p cosmol_viewer_wasm --locked --no-default-features --test binding_contract --test scene
```
