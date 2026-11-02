# COSMol Viewer
A high-performance molecular viewer for Python and Rust, powered by a unified Rust core.
It supports both in-notebook visualization and native desktop rendering, with smooth playback for scientific animations.

Binding developers: see [DEV.md](DEV.md) for the official PyO3 type extraction
workflow and [BINDING_CONTRACT.md](BINDING_CONTRACT.md) for extension rules.

Full installation, workflow, and Python API documentation is available at
<https://cosmol-studio.github.io/COSMol-viewer/>.

<div align="center">
  <a href="https://pypi.org/project/cosmol-viewer/">
    <img src="https://img.shields.io/pypi/v/cosmol-viewer.svg" alt="PyPi Latest Release" />
  </a>
  <a href="https://cosmol-studio.github.io/COSMol-viewer">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg" alt="Documentation Status" />
  </a>
  <a href="https://crates.io/crates/cosmol_viewer">
    <img src="https://img.shields.io/crates/v/cosmol_viewer.svg" alt="crates.io Latest Release" />
  </a>
  <a href="https://www.npmjs.com/package/cosmol_viewer_wasm">
    <img src="https://img.shields.io/npm/v/cosmol_viewer_wasm.svg" alt="npm Latest Release" />
  </a>
</div>

COSMol Viewer is a compact, cross-platform renderer for molecular and geometric scenes.
Unlike purely notebook-bound solutions such as py3Dmol, COSMol Viewer runs everywhere:

- Native desktop window (Python or Rust) via `egui`
- Jupyter / IPython notebook via WASM backend
- Rust applications

All implementations share the same Rust rendering engine, ensuring consistent performance and visual output.

---

## Quick concepts

- **Scene**: container for shapes (molecules, proteins, spheres, etc.).
- **Viewer.render(scene, ...)**: create an interactive viewer in a native window or notebook canvas.
- **scene.save_image(path, ...)** / **scene.to_png(...)** / **scene.display(...)**: render the scene directly to a static PNG at any requested resolution. This is independent of notebook JavaScript or browser canvas readback.
- **scene.set_camera_view(...)** / **scene.rotate_camera(...)**: set the reproducible camera used by both static exports and newly created viewers.
- **viewer.update(scene)**: push incremental changes after `Viewer.render()` (real-time / streaming use-cases).
- **Animation**: An Animation object containing frames and settings.
- **Animation(interval, loops, interpolate)**: stores precomputed frames and playback settings.
- **Viewer.play(animation, width, height)**: *recommended* for precomputed animations and demonstrations. The viewer takes care of playback timing and looping.

**Why prefer `play` for demos?**
- Single call API (hand off responsibility to the viewer).
- Built-in timing & loop control.
- Optional `interpolate` mode between frames for visually pleasing playback even when input frame rate is low.

**Why keep `update`?**
- `update` is ideal for real-time simulations, MD runs, or streaming data where frames are not precomputed. It provides strict fidelity (no interpolation) and minimal latency.

---

# Usage

## python
See examples in [Google Colab](https://colab.research.google.com/drive/1Sw72QWjQh_sbbY43jGyBOfF1AQCycmIx?usp=sharing).

Install with `pip install cosmol-viewer`

### 1. Static molecular rendering

```python
import cosmolkit as ck
from cosmol_viewer import BallAndStick, Scene, select

mol_data = open("molecule.sdf", "r", encoding="utf-8").read()

mol = ck.Molecule.from_sdf(mol_data)

scene = Scene()

scene.set_scale(1.0)

scene.show_with_id("molecule", mol, select(mol).atoms(), BallAndStick())
scene.recenter_on(mol)

scene.set_camera_view(azimuth=35, elevation=20, distance=32, fov=18)
scene.save_image("rendered_scene.png", width=1600, height=1000)
```

Static exports and native interactive viewers both bootstrap native GL on
desktop. `save_image` / `to_png` first try the fast in-process offscreen path;
on platforms with a headless GL path, this avoids creating a GUI event loop. If
the in-process path cannot be created after a native viewer has already run,
they automatically retry in an isolated Python subprocess. Set
`COSMOL_VIEWER_RENDER_ISOLATED=1` to force that isolated path. Google Colab is
detected automatically and uses an isolated software-rendering subprocess, so
notebook code does not need to set either this variable or
`LIBGL_ALWAYS_SOFTWARE`.

For an interactive native window:

```python
from cosmol_viewer import Viewer

viewer = Viewer.render(scene, width=800, height=500)

viewer.keep_alive()
```

`keep_alive()` waits for Enter, stdin EOF, or the native viewer process to exit,
then closes the viewer. It is a finalization step: the viewer cannot be reused
afterwards. Calling it in Jupyter or Colab raises `RuntimeError`.

Native viewers run in a dedicated executable bundled with the Python wheel.
No `register_render()` call is needed in Python. For a source development build,
see [DEV.md](DEV.md).

In a notebook, use a static PNG display when you do not need interaction:

```python
scene.display(width=1200, height=800)
scene.display(width=1200, height=800, background="transparent")
```

For an interactive notebook canvas, enable a transparent scene background before
rendering:

```python
scene.set_transparent_background()
scene.set_zoom_disabled()
scene.set_auto_rotate()
viewer = Viewer.render(scene, width=800, height=500)
```

For static exports, omit `background` to use the scene background, pass a color
such as `"#ffffff"` or `[255, 255, 255]`, or use `"transparent"` for a PNG with
a transparent background.

### 2. Animation playback with `Viewer.play`

Scientific objects stay in CK. Selections reject a different source object or
changed rendering inputs; select again after an in-place edit. Coordinates must
already exist. For extra molecular representations use `SphereStyle` or
`StickStyle`; `Sphere`/`Stick` remain positioned primitives.

```python
from cosmol_viewer import SphereStyle
oxygen = select(mol).atoms().element(ck.Element.O)
scene.show(mol, oxygen, SphereStyle().scale(1.4))
```

```python
import cosmolkit as ck
from cosmol_viewer import Scene, Viewer, BallAndStick, Animation, select

anim = Animation(interval=0.05, loops=-1, interpolate=False)
for i in range(1, 10):
    with open(f"frames/frame_{i}.sdf", "r") as f:
        mol = ck.Molecule.from_sdf(f.read())

    scene = Scene()
    scene.show(mol, select(mol).atoms(), BallAndStick())
    anim.add_frame(scene)

Viewer.play(anim, width=800, height=500)  # loops=-1 for infinite repeat
```

### 3. Protein cartoon rendering

```python
import cosmolkit as ck
from cosmol_viewer import Cartoon, Scene, Viewer, select

mmcif_data = open("protein.cif", "r", encoding="utf-8").read()
protein = ck.Protein.from_mmcif(mmcif_data)

scene = Scene()
scene.show_with_id("protein", protein, select(protein).residues(), Cartoon().rainbow_residues())
scene.recenter_on(protein)

viewer = Viewer.render(scene, width=800, height=500)
```

`ck.Protein.from_mmcif()` and `ck.Protein.from_pdb()` create the scientific
object; the viewer core assigns secondary structure before rendering a
ChimeraX-style cartoon ribbon mesh. Use
`Cartoon().rainbow_residues()` for ChimeraX-style residue rainbow coloring, or `Cartoon().color("#10ACBF")`
for a uniform cartoon color.

more examples can be found in the [examples](https://github.com/COSMol-repl/COSMol-viewer/tree/main/cosmol_viewer/examples) folder:
```bash
cd cosmol_viewer
python .\examples\render_protein.py
```

# Documentation

Please check out our documentation at [here](https://cosmol-studio.github.io/COSMol-viewer/).

---

# Contact

For any questions, issues, or suggestions, please contact [wjt@cosmol.org](mailto:wjt@cosmol.org) or open an issue in the repository. We will review and address them as promptly as possible.
