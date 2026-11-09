from pathlib import Path

import cosmolkit as ck
from cosmol_viewer import BallAndStick, Cartoon, Scene, Viewer

mmcif_data = Path(__file__).with_name("6fi1.cif").read_text(encoding="utf-8")
prot = ck.Protein.from_mmcif(mmcif_data)

ligand_data = Path(__file__).with_name("6fi1_ligand.sdf").read_text(encoding="utf-8")
ligand = ck.Molecule.from_sdf(ligand_data)

scene = Scene()
scene.set_scale(0.8)
scene.recenter_on(ligand)
scene.set("prot", prot, Cartoon().rainbow_residues())
scene.set("ligand", ligand, BallAndStick().outline("#EEEEEE", 0.04))
scene.set_background_color("#021529")
scene.set_camera_view(azimuth=-90, elevation=-30, distance=28, fov=18)
scene.save_image("protein_ligand.png", width=1200, height=900)

viewer = Viewer.show(scene, width=800, height=500)

viewer.show_fps(True)
viewer.show_camera_parameters(True)
viewer.keep_alive()
