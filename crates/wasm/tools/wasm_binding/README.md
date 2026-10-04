# Browser Binding Pipeline

Python stays on PyO3 and `pyo3-stub-gen`. Browser JavaScript uses Alef **0.103.12**
to extract `bindings.rs`, `scene.rs`, and `notebook.rs`; wasm-pack produces the
JavaScript and `.d.ts`. Do not build the npm package directly from `crates/wasm`.

From the repository root (Python 3.11+, Node, wasm-pack, and Alef on PATH):

```sh
python crates/wasm/tools/wasm_binding/build.py --check
```

On Windows, activate `conda activate COS` first. `ALEF_BIN`, `WASM_PACK_BIN`,
`NODE_BIN`, and `NPX_BIN` override executable discovery. `--generate-only`
validates generation without replacing the package. `--no-opt` skips wasm-opt
for development; releases retain the flags from `crates/wasm/Cargo.toml`.

Generation happens in a temporary workspace, sharing the repository Cargo cache.
Only successfully built (and, with `--check`, tested) packages are staged in
`crates/wasm/pkg`. Versions come from the workspace manifest.

The pinned Alef generator currently drops async on static methods and treats
borrowed opaque arguments as owned values. `compat.py` corrects these two cases
using Alef's extracted API metadata, without another handwritten signature list.
Regression tests check these transformations; unexpected output fails the build.

## Typed JavaScript API

The npm package is `@cosmol-studio/cosmol-viewer`. Install prereleases with
`npm install @cosmol-studio/cosmol-viewer@rc`; stable releases use the default
`latest` tag. The package name does not rename the Rust crate or generated
`cosmol_viewer_wasm.js` / `.wasm` files.

`Viewer` owns browser renderer state directly. `Scene` and `Animation` are
reusable handles over the existing core models, not compressed payload strings.

```js
import init, { Viewer, Scene, Animation } from "@cosmol-studio/cosmol-viewer";

await init();
const scene = Scene.new();
scene.addSphere("atom", new Float32Array([0, 0, 0]), 1.0);
const viewer = await Viewer.render(scene, 800, 600);

scene.setScale(1.2);
viewer.update(scene);
const png = await viewer.takeScreenshot(); // Uint8Array of PNG bytes

const animation = Animation.new(0.1, -1n, true);
animation.addFrame(scene);
scene.setScale(1.4);
animation.addFrame(scene);
const playing = await Viewer.play(animation, 800, 600);

viewer.close();
playing.close();
viewer.free();
playing.free();
animation.free();
scene.free();
```

- `render` / `play` create a canvas in `document.body`. `close()` destroys the
  renderer and removes that canvas; `free()` releases the binding handle.
- `renderInto(canvasId, scene)` / `playInto(canvasId, animation)` use an existing
  canvas without changing its size. `close()` leaves caller-owned canvases intact.
- Borrowed scenes/animations remain usable after render, update, or addFrame.
  Frames and the static scene capture snapshots, matching the core API.
- Cloned binding handles share state; `scene.cloneScene()` explicitly makes an
  independent scene. Render/update snapshot state and prepare GPU data internally.
- Coordinates use `Float32Array`; RGB colors use `Uint8Array`; loop counts use
  `bigint`. Async failures reject, and synchronous failures throw.
- Scene supports camera, lighting, backgrounds, depth cue, sphere/stick creation,
  merging and removal. Dedicated molecule/protein shape bindings remain future
  work; this change does not claim all scientific-object APIs are exported.

## Notebook transport is separate

Notebook HTML calls `Viewer.renderNotebook(canvasId, compressedScene)` or
`Viewer.playNotebook(canvasId, compressedAnimation)`. These internal transport
endpoints decode Python payloads and enter the same typed render/play paths.
Notebook updates and camera logging are compressed one-way `ViewerCommand`
messages submitted to `viewer.dispatch(payload)`.

Normal JS callers use `update(scene)` / `setCameraParameterLogging(enabled)`;
they do not serialize commands or scenes. `takeScreenshot()` returns PNG bytes;
`takeScreenshotNotebook()` retains the compressed notebook RPC response format.
Internal notebook endpoints must remain exported for injected notebook scripts,
but are not the primary typed API.

## Contracts and platform boundaries

`crates/core/src/binding_contract/registry.rs` registers semantic features, exact
Rust signatures, Python/JavaScript projections, inspectable defaults, notebook
endpoints/commands, and platform availability. Rust signatures are checked by
the compiler. The same metadata is available through Rust
`binding_contract::to_json()`, Python `binding_contract_json()`, and browser
`bindingContractJson()`. Registration does not generate PyO3 behavior or require
identical language ABIs. `Viewer.keep_alive` is native-only: notebooks reject it,
and JavaScript does not export it.

The derive helper is a versioned dependency of the public core crate. For a Rust
release, publish `cosmol_viewer_derive` before `cosmol_viewer_core` and the facade.
This build script never publishes packages.

## Test boundaries

- `build.py --check` runs the generated WASM in Node, verifies exports, ownership,
  snapshots, error paths and exclusions, then checks strict TypeScript.
- `build.py --check --browser-check` also renders in real headless Chromium:
  render/update/play, PNG screenshots, and owned/caller-owned canvas cleanup.
- Python binding tests check registered APIs and generated stubs. Notebook tests
  mock Jupyter/Colab display, decode real Python startup/command payloads in WASM,
  and check pre-initialization errors. They are not a full notebook UI test.

Install the optional browser test dependencies from the repository root:

```sh
npm install --prefix target/browser-check --no-save playwright@1.58.2
target/browser-check/node_modules/.bin/playwright install chromium
python crates/wasm/tools/wasm_binding/build.py --check --browser-check
```

Linux CI uses `playwright install --with-deps chromium`. `PLAYWRIGHT_MODULE` can
point to another installed Playwright module and `BROWSER_EXECUTABLE` to a local
Chromium/Chrome executable. These test dependencies are not shipped in npm/wheels.
