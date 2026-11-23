"""Show flat element colors with depth-based silhouettes."""
from pathlib import Path

from cosmolkit import Molecule
from cosmol_viewer import BallAndStick, MoleculeColors, Scene, Viewer

ligand_file = Path(__file__).with_name("6fi1_ligand.sdf")
ligand = Molecule.from_sdf(ligand_file.read_text(encoding="utf-8")).with_kekulized_bonds()
colors = MoleculeColors({"C": "#FFC532"})

scene = Scene()
scene.set("ligand", ligand, BallAndStick().colors(colors))
scene.recenter_on(ligand)
scene.set_camera_view(azimuth=-55, elevation=20, distance=22, fov=22)
scene.set_background_color("#FFFFFF")
# Constant white ambient light preserves the element colors.
scene.set_lighting(ambient=1.0, diffuse=0.0, specular=0.0, color="#FFFFFF")
scene.set_silhouettes(color="#404040", width=4, depth_jump=0.03)

scene.save_image("flat_lighting.png", width=1200, height=900)
Viewer.show(scene, width=800, height=600).keep_alive()
