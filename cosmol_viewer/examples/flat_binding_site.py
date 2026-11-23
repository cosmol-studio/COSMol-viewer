"""Render the 1IEP binding site with flat colors, silhouettes, and depth cueing."""
from pathlib import Path

from cosmolkit import Molecule, Protein
from cosmol_viewer import Cartoon, MoleculeColors, Scene, StickStyle, Viewer

assets = Path(__file__).resolve().parents[2] / "docs-web/data/showcase"
protein = Protein.read(str(assets / "1iep-receptor.pdb"))
poses = (assets / "imatinib-poses.sdf").read_text(encoding="utf-8")
# The first SDF record is the crystallographic ligand.
ligand = Molecule.from_sdf(poses.partition("$$$$")[0] + "$$$$\n")
colors = MoleculeColors({"C": "#A5FFA1", "N": "#435DF2", "O": "#A5FFA1"})
outline = "#404040"

scene = Scene()
scene.set("receptor", protein, Cartoon().color("#74ACFF").silhouettes(outline, 4))
sticks = (
    StickStyle().show_bond_order(False).scale(1.45).silhouettes(outline, 4).colors(colors)
)
scene.set("ligand", ligand, sticks)
scene.recenter_on(ligand)
scene.set_scale(0.8)
scene.set_camera_view(azimuth=77, elevation=-22, roll=53, distance=34, fov=22)
scene.set_background_color("#FFFFFF")
scene.set_lighting(ambient=1.0, diffuse=0.0, specular=0.0, color="#FFFFFF")
scene.set_depth_cue(True)

scene.save_image("flat_binding_site.png", width=1400, height=840)
Viewer.show(scene, width=1000, height=600).keep_alive()
