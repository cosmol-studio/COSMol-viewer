# JavaScript / WASM bindings

Rust declarations own JS names, signatures, documentation and ownership.
`#[checked_wasm]` generates typed Tsify/Serde input adapters from those declarations;
structured values share their runtime converters and generated TypeScript types. wasm-pack builds this crate directly and runs the matching official
wasm-bindgen extractor. Its JavaScript and `.d.ts` output is used unchanged.
See [BINDING_CONTRACT.md](BINDING_CONTRACT.md) for architectural constraints and
[DEV.md](DEV.md) for build commands.

## Typed JavaScript API

The npm package is `@cosmol-studio/cosmol-viewer`. Install prereleases with
`npm install @cosmol-studio/cosmol-viewer@rc`; stable releases use the default
`latest` tag. The package name does not rename the Rust crate or generated
`cosmol_viewer_wasm.js` / `.wasm` files.

`Viewer` owns the browser runner and a request/feedback transport endpoint.
The runner exclusively owns the core `App`; viewer handles do not share or lock
it. `Scene` and `Animation` are reusable handles over the existing core models,
not compressed payload strings.

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
- `update` and camera logging queue typed requests and return without blocking.
  Native IPC and browser queues enter the same core `App` message handler.
  Browser queues wake the renderer instead of polling continuously.
- Screenshot requests are FIFO barriers: later updates wait until capture.
  Async screenshot calls share a request-ID counter across cloned handles, so
  concurrent replies cannot overwrite or consume each other's images. Closing
  the viewer rejects pending screenshot waits.
- Coordinates use `Float32Array`; RGB colors use `Uint8Array`; loop counts use
  `bigint`. Async failures reject, and synchronous failures throw.
- Scene supports camera, lighting, backgrounds, depth cue, sphere/stick creation,
  merging and removal. Dedicated molecule/protein shape bindings remain future
  work; this change does not claim all scientific-object APIs are exported.

## Notebook transport is separate

Notebook HTML calls `Viewer.renderNotebook(canvasId, compressedScene)` or
`Viewer.playNotebook(canvasId, compressedAnimation)`. These internal transport
endpoints decode Python payloads and enter the same typed render/play paths.
Notebook updates, camera logging and viewport overlays are encoded one-way `ViewerCommand`
messages submitted to `viewer.dispatch(payload)`.

Scenes, animations, and commands share the `CMV2:R:<base64>` (raw postcard) or
`CMV2:G:<base64>` (gzip postcard) format. Postcard data below 1 KiB skips gzip;
at or above 1 KiB gzip level 1 is always used. The choice depends only on the
serialized size; there is no compression-ratio check or fallback.
Old unprefixed and CMV1 formats are rejected: sender and receiver must match.
The decoder defaults to at most 64 MiB of serialized data; Rust callers can use
`decode_payload_with_limit` to set another bound. This is a serialized-byte limit,
not a limit on the final object or GPU memory. Ordinary JS and native IPC do not
use this notebook codec.

Normal JS callers use `update(scene)`, `cameraParameterLogging(enabled)`,
`showFps(enabled)` and `showCameraParameters(enabled)`;
they do not serialize commands or scenes. `takeScreenshot()` returns PNG bytes
to JavaScript only. Notebook transport is one-way: Colab/Jupyter return-value
RPC and notebook screenshot retrieval are not supported.
Internal notebook endpoints must remain exported for injected notebook scripts,
but are not the primary typed API.

Overlays are initially hidden, do not capture pointer input, and never request
continuous repainting. FPS samples existing renderer repaint events (not animation
frames or GPU timing). After one second without activity, one diagnostic repaint
shows zero FPS. Diagnostic/screenshot-only paints do not count as activity or
renew the idle timer; interaction or scene updates resume statistics.

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
python wasm/build.py --check --browser-check
```

Linux CI uses `playwright install --with-deps chromium`. `PLAYWRIGHT_MODULE` can
point to another installed Playwright module and `BROWSER_EXECUTABLE` to a local
Chromium/Chrome executable. These test dependencies are not shipped in npm/wheels.
