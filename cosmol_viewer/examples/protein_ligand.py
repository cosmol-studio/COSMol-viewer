from cosmol_viewer import Molecule, Protein, Scene, Viewer

mmcif_data = open("./examples/6fi1.cif", "r", encoding="utf-8").read()
prot = Protein.from_mmcif(mmcif_data).rainbow_residues()

ligand_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()
ligand = Molecule.from_sdf(ligand_data).set_outline(True, "#EEEEEE", 0.04)

scene = Scene()
scene.set_scale(0.8)
scene.recenter(ligand.get_center())
scene.add_shape_with_id("prot", prot)
scene.add_shape_with_id("ligand", ligand)
scene.set_background_color("#021529")
scene.set_camera_view(azimuth=-90, elevation=-30, distance=28, fov=18)
scene.save_image("protein_ligand.png", width=1200, height=900)

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
try:
    _ = input()
except EOFError:
    pass
