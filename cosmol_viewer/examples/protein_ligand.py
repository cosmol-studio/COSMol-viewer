import cosmolkit as ck
from cosmol_viewer import BallAndStick, Cartoon, Scene, Viewer, select

mmcif_data = open("./examples/6fi1.cif", "r", encoding="utf-8").read()
prot = ck.Protein.from_mmcif(mmcif_data)

ligand_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()
ligand = ck.Molecule.from_sdf(ligand_data)

scene = Scene()
scene.set_scale(0.8)
scene.recenter_on(ligand)
scene.show_with_id("prot", prot, select(prot).residues(), Cartoon().rainbow_residues())
scene.show_with_id("ligand", ligand, select(ligand).atoms(), BallAndStick().outline("#EEEEEE", 0.04))
scene.set_background_color("#021529")
scene.set_camera_view(azimuth=-90, elevation=-30, distance=28, fov=18)
scene.save_image("protein_ligand.png", width=1200, height=900)

viewer = Viewer.render(scene, width=800, height=500)

viewer.show_fps(True)
viewer.show_camera_parameters(True)
viewer.keep_alive()
