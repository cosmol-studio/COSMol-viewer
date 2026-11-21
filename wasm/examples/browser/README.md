# Browser example

Before running the commands below, complete the [WASM build and npm package preparation](../../DEV.md).

A plain JavaScript example installing both Viewer and COSMolKit through
`package.json`. No frontend framework or bundler is needed.

From the repository root:

```sh
python wasm/check.py
cd wasm/examples/browser
npm install
npm start
```

Viewer currently uses `"@cosmol-studio/cosmol-viewer": "file:../../pkg"` because
these bindings are not published yet. The project's `.npmrc` enables
`install-links=true`: npm packs and installs the local package as a regular
dependency, honoring its published file list instead of creating a symlink.
Both JavaScript and WASM are loaded from `node_modules`; the page never imports
Viewer directly from the build directory. After publishing, replace the
`file:` dependency with the released version.

After rebuilding Viewer locally, run `npm ci` in this example directory to
reinstall the current package contents.

Open <http://127.0.0.1:8000/wasm/examples/browser/>.
Activate `conda activate COS` first when using the project's Windows environment.
The server must serve the repository root; opening the HTML with `file://` will
not load the WASM modules and scientific data.

The page demonstrates:

- **Molecule:** a CK molecule, atom/element selections and multiple display styles.
- **Molecular colors:** four ligand copies compare a scene palette, shared
  yellow atom colors, and a separate pink scheme. Use **Restore built-in
  colors** to reset the scene and replace the four displays, then **Apply
  custom colors** to switch back. The Rust and Python `molecule_colors`
  examples use the same layout and export both versions offscreen.
- **Protein + ligand:** CK Protein and Molecule objects in the same scene, with
  a residue cartoon and molecular ball-and-stick display.
- **Highlight residues:** continuous Cartoon ribbon with residues 10?14 in red,
  using `residues().filterIds(...)` and `difference`, matching the Rust and Python
  `highlight_residues` examples. The range uses zero-based CK IDs, not CIF residue numbers.
- **Protein side chains:** whole-protein stick geometry with default element colors with separately colored
  side-chain heavy atoms, using `residueIds`, `sidechain` and `difference`.
- **Animation:** 32 scene snapshots played with `Animation` and `viewer.play(animation)`.
- Named display replacement with `set`, followed by `viewer.update(scene)`.
- `scene.recenterOn(molecule)`, FPS/camera overlays and PNG screenshots.

The scientific inputs reuse `cosmol_viewer/examples/6fi1.cif` and
`6fi1_ligand.sdf`. CK and Viewer initialize their WASM modules on import. The
HTML import map makes the example and Viewer resolve the same CK entry point, so foreign
object identity checks work without copying or patching generated bindings.

See [main.js](main.js) for the API calls. Switching examples closes the previous
renderer and releases its WASM handles.
