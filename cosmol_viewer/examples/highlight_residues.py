"""Highlight residues on a continuous Cartoon ribbon; export offscreen and open a viewer."""
from pathlib import Path

from cosmolkit import Protein
from cosmol_viewer import Cartoon, Scene, Viewer, select

protein = Protein.from_mmcif(Path(__file__).with_name("6fi1.cif").read_text(encoding="utf-8"))
residues = select(protein).residues()
# Zero-based CK residue IDs, not the residue numbers in the CIF.
highlighted = residues.filter_ids(list(range(10, 15)))
remaining = residues.difference(highlighted)

scene = Scene()
# Complementary selections share complete-chain Cartoon geometry.
scene.set("remaining", remaining, Cartoon().color("#AAB4C0"))
scene.set("highlighted", highlighted, Cartoon().color("#FF4444"))
scene.recenter_on(protein)
scene.set_scale(0.2)
scene.set_background_color("#021529")

output = Path(__file__).resolve().parents[1] / "highlight_residues.png"
scene.save_image(str(output), width=1200, height=900)
print(f"Highlighted {len(highlighted)} residues; saved {output}")
viewer = Viewer.show(scene, width=800, height=500)
viewer.keep_alive()
