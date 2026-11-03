from pathlib import Path

import cosmolkit as ck
from cosmol_viewer import StickStyle, Scene, Viewer

mol_data = Path(__file__).with_name("6fi1_ligand.sdf").read_text(encoding="utf-8")

mol = ck.Molecule.from_sdf(mol_data)

scene = Scene()
scene.set_scale(1.2)

scene.set("molecule", mol, StickStyle().outline("#EEEEEE", 0.04))
scene.recenter_on(mol)
scene.set_camera_view(azimuth=-55, elevation=20, distance=32, fov=18)

scene.save_image("render_molecule_stick.png", width=1200, height=900)

viewer = Viewer.open(scene, width=800, height=500)

viewer.keep_alive()
