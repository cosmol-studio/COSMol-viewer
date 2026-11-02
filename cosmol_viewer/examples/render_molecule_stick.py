import cosmolkit as ck
from cosmol_viewer import StickStyle, Scene, Viewer, select

mol_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()

mol = ck.Molecule.from_sdf(mol_data)

scene = Scene()
scene.set_scale(1.2)

scene.show_with_id("molecule", mol, select(mol).atoms(), StickStyle().outline("#EEEEEE", 0.04))
scene.recenter_on(mol)
scene.set_camera_view(azimuth=-55, elevation=20, distance=32, fov=18)

scene.save_image("render_molecule_stick.png", width=1200, height=900)

viewer = Viewer.render(scene, width=800, height=500)

viewer.keep_alive()
