"""Compare scene silhouettes on a protein cartoon, ligand, and transparent surface."""

from pathlib import Path

from cosmolkit import Molecule, Protein
from cosmol_viewer import Cartoon, Scene, StickStyle, Surface, Viewer, select

examples = Path(__file__).resolve().parent
protein = Protein.from_mmcif((examples / "6fi1.cif").read_text(encoding="utf-8"))
sdf = (examples / "6fi1_ligand.sdf").read_text(encoding="utf-8")
ligand = Molecule.from_sdf(sdf).with_kekulized_bonds()

scene = Scene()
scene.set_background_color("#FFFFFF")
scene.set_lighting(ambient=0.55, diffuse=0.55, specular=0.14, color="#FFFFFF")
scene.recenter_on(protein)
scene.set_scale(0.18)
scene.set_camera_view(azimuth=-90, elevation=-20, distance=32, fov=15)
scene.set("protein", protein, Cartoon().color("#FFFFFF").roughness(0.85))
scene.set("ligand", ligand, StickStyle())

output = examples.parent
scene.save_image(output / "scene_silhouettes_off.png", width=1280, height=800)

# A single scene setting draws edges for both the ribbon and the ligand.
scene.set_silhouettes(color="#505050", width=4)
scene.save_image(output / "scene_silhouettes_on.png", width=1280, height=800)

# The ribbon and ligand outlines remain visible through the surface.
shell = Surface().color("#A9CBE2").opacity(0.30)
scene.set("surface", select(protein).atoms(), shell)
scene.save_image(output / "scene_silhouettes_surface.png", width=1280, height=800)
print(f"Saved three scene_silhouettes PNGs in {output}")

Viewer.show(scene, width=800, height=500).keep_alive()
