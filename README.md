# COSMol-viewer
A high-performance molecular viewer for Python and Rust, powered by a unified Rust core.
It supports both in-notebook visualization and native desktop rendering, with smooth playback for scientific animations.

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

COSMol-viewer is a compact, cross-platform renderer for molecular and geometric scenes.
Unlike purely notebook-bound solutions such as py3Dmol, COSMol-viewer runs everywhere:

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
from cosmol_viewer import Molecule, Scene

mol_data = open("molecule.sdf", "r", encoding="utf-8").read()

mol = Molecule.from_sdf(mol_data).centered()

scene = Scene()

scene.set_scale(1.0)

scene.add_shape_with_id("molecule", mol)

scene.set_camera_view(azimuth=35, elevation=20, distance=32, fov=18)
scene.save_image("rendered_scene.png", width=1600, height=1000)
```

Static exports and native interactive viewers both bootstrap native GL on
desktop. Until the offscreen backend is fully winit-free, use `save_image` /
`to_png` / `display` as a static workflow, or use `Viewer.render` as an
interactive workflow; do not call static export immediately before
`Viewer.render` in the same native process.

For an interactive native window:

```python
from cosmol_viewer import Viewer

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end='', flush=True)
_ = input()
```

In a notebook, use a static PNG display when you do not need interaction:

```python
scene.display(width=1200, height=800)
scene.display(width=1200, height=800, background="transparent")
```

For an interactive notebook canvas, enable a transparent scene background before
rendering:

```python
scene.set_transparent_background()
viewer = Viewer.render(scene, width=800, height=500)
```

For static exports, omit `background` to use the scene background, pass a color
such as `"#ffffff"` or `[255, 255, 255]`, or use `"transparent"` for a PNG with
a transparent background.

### 2. Animation playback with `Viewer.play`

```python
from cosmol_viewer import Scene, Viewer, Molecule, Animation

anim = Animation(interval=0.05, loops=-1, interpolate=False)
for i in range(1, 10):
    with open(f"frames/frame_{i}.sdf", "r") as f:
        mol = Molecule.from_sdf(f.read())

    scene = Scene()
    scene.add_shape(mol)
    anim.add_frame(scene)

Viewer.play(anim, width=800, height=500)  # loops=-1 for infinite repeat
```

### 3. Protein cartoon rendering

```python
from cosmol_viewer import Protein, Scene, Viewer

mmcif_data = open("protein.cif", "r", encoding="utf-8").read()
protein = Protein.from_mmcif(mmcif_data).centered().rainbow_residues()

scene = Scene()
scene.add_shape_with_id("protein", protein)

viewer = Viewer.render(scene, width=800, height=500)
```

`Protein.from_mmcif()` and `Protein.from_pdb()` use COSMolKit's protein
reader, then the viewer core assigns secondary structure before rendering a
ChimeraX-style cartoon ribbon mesh. Use
`.rainbow_residues()` for ChimeraX-style residue rainbow coloring, or `.color("#10ACBF")`
for a uniform cartoon color.

more examples can be found in the [examples](https://github.com/COSMol-repl/COSMol-viewer/tree/main/cosmol_viewer/examples) folder:
```bash
cd cosmol_viewer
python .\examples\render_protein.py
```

## Rust

Install with `cargo add cosmol_viewer`

see examples in [examples](https://github.com/COSMol-repl/COSMol-viewer/tree/main/cosmol_viewer/examples) folder:
```bash
cd cosmol_viewer
cargo run --example render_protein
```

# Documentation

Please check out our documentation at [here](https://cosmol-studio.github.io/COSMol-viewer/).

---

# Contact

For any questions, issues, or suggestions, please contact [wjt@cosmol.org](mailto:wjt@cosmol.org) or open an issue in the repository. We will review and address them as promptly as possible.
