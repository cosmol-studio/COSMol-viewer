# Browser example

A plain JavaScript example installing both Viewer and COSMolKit through
`package.json`. No frontend framework or bundler is needed.

From the repository root:

```sh
python wasm/build.py --check
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
- **Protein + ligand:** CK Protein and Molecule objects in the same scene, with
  a residue cartoon and molecular ball-and-stick display.
- **Animation:** 32 scene snapshots played with `Animation` and `Viewer.playInto`.
- Named display replacement with `showWithId`, followed by `viewer.update(scene)`.
- `scene.recenterOn(molecule)`, FPS/camera overlays and PNG screenshots.

The scientific inputs reuse `cosmol_viewer/examples/6fi1.cif` and
`6fi1_ligand.sdf`. CK and Viewer are initialized separately. The HTML import map
makes the example and Viewer resolve exactly the same CK module, so foreign
object identity checks work without copying or patching generated bindings.

See [main.js](main.js) for the API calls. Switching examples closes the previous
renderer and releases its WASM handles.
