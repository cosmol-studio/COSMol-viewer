from pathlib import Path

import cosmolkit as ck
from cosmol_viewer import BallAndStick, Scene, Viewer

mol_data = Path(__file__).with_name("6fi1_ligand.sdf").read_text(encoding="utf-8")

mol = ck.Molecule.from_sdf(mol_data)

scene = Scene()

scene.set("molecule", mol, BallAndStick().outline("#EEEEEE", 0.04))
scene.recenter_on(mol)
scene.set_camera_view(azimuth=35, elevation=20, distance=32, fov=18)

scene.save_image("render_molecule_offscreen.png", width=800, height=500)

viewer = Viewer.open(scene, width=800, height=500)

viewer.keep_alive()
