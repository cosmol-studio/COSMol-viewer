import cosmolkit as ck
from cosmol_viewer import BallAndStick, Scene, Viewer, select

mol_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()

mol = ck.Molecule.from_sdf(mol_data)

scene = Scene()

scene.show_with_id("molecule", mol, select(mol).atoms(), BallAndStick().outline("#EEEEEE", 0.04))
scene.recenter_on(mol)
scene.set_camera_view(azimuth=35, elevation=20, distance=32, fov=18)

scene.save_image("render_molecule_offscreen.png", width=800, height=500)

viewer = Viewer.render(scene, width=800, height=500)

viewer.keep_alive()
