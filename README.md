# COSMol-viewer
A high-performance molecular viewer for Python and Rust, powered by a unified Rust core.
It supports both in-notebook visualization and native desktop rendering, with smooth playback for scientific animations.

<div align="center">
  <a href="https://crates.io/crates/cosmol_viewer">
    <img src="https://img.shields.io/crates/v/cosmol_viewer.svg" alt="crates.io Latest Release" />
  </a>
  <a href="https://pypi.org/project/cosmol_viewer/">
    <img src="https://img.shields.io/pypi/v/cosmol_viewer.svg" alt="PyPi Latest Release" />
  </a>
  <a href="https://cosmol-repl.github.io/COSMol-viewer">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg" alt="Documentation Status" />
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
- **Viewer.render(scene, ...)**: create a static viewer bound to a canvas (native or notebook). Good for static visualization.
- **viewer.update(scene)**: push incremental changes after `Viewer.render()` (real-time / streaming use-cases).
- **Animation**: An Animation object containing frames and settings.
- **Viewer.play(animation, interval, loops, width, height, smooth)**: *recommended* for precomputed animations and demonstrations. The viewer takes care of playback timing and looping.

**Why prefer `play` for demos?**
- Single call API (hand off responsibility to the viewer).
- Built-in timing & loop control.
- Optional `smooth` interpolation between frames for visually pleasing playback even when input frame rate is low.

**Why keep `update`?**
- `update` is ideal for real-time simulations, MD runs, or streaming data where frames are not precomputed. It provides strict fidelity (no interpolation) and minimal latency.

---

# Usage

## python
See examples in [Google Colab](https://colab.research.google.com/drive/1Sw72QWjQh_sbbY43jGyBOfF1AQCycmIx?usp=sharing).

Install with `pip install cosmol-viewer`

### 1. Static molecular rendering

```python
from cosmol_viewer import Molecule, Scene, Viewer

mol_data = open("molecule.sdf", "r", encoding="utf-8").read()

mol = Molecule.from_sdf(mol_data).centered()

scene = Scene()

scene.set_scale(1.0)

scene.add_shape_with_id("molecule", mol)

viewer = Viewer.render(scene, width=800, height=500)

viewer.save_image("screenshot.png")

print("Press Any Key to exit...", end='', flush=True)
_ = input()  # Keep the viewer open until you decide to close
```

### 2. Animation playback with `Viewer.play`

```python
from cosmol_viewer import Scene, Viewer, Molecule, Animation

anim = Animation(interval=0.05, loops=-1, smooth=False)
for i in range(1, 10):
    with open(f"frames/frame_{i}.sdf", "r") as f:
        mol = Molecule.from_sdf(f.read())

    scene = Scene()
    scene.add_shape(mol)
    anim.add_frame(scene)

Viewer.play(anim, width=800, height=500) # loops=-1 for infinite repeat
```

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
