"""Display a whole protein as sticks, highlighting selected side-chain heavy atoms."""
from pathlib import Path

from cosmolkit import Protein
from cosmol_viewer import Scene, StickStyle, Viewer, select

protein = Protein.from_mmcif(Path(__file__).with_name("6fi1.cif").read_text(encoding="utf-8"))
atoms = select(protein).atoms()
# Zero-based CK residue IDs, not PDB sequence numbers. Hydrogens are excluded.
highlighted = atoms.residue_ids([10, 11, 12, 13, 14]).sidechain()
rest = atoms.difference(highlighted)

scene = Scene()
scene.set("protein", rest, StickStyle())
scene.set("sidechains", highlighted, StickStyle().color("#FF4444"))
scene.recenter_on(protein)
scene.set_scale(0.2)
scene.set_background_color("#021529")
output = Path(__file__).resolve().parents[1] / "protein_sticks.png"
scene.save_image(str(output), width=1200, height=900)
viewer = Viewer.show(scene, width=800, height=600)
viewer.keep_alive()
