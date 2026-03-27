from cosmol_viewer import Molecule, Protein, Scene, Viewer

mmcif_data = open("./examples/6fi1.cif", "r", encoding="utf-8").read()
prot = Protein.from_mmcif(mmcif_data).color("#10ACBf")

ligand_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()
ligand = Molecule.from_sdf(ligand_data)

scene = Scene()
scene.add_shape_with_id("prot", prot)
scene.add_shape_with_id("ligand", ligand)
scene.recenter(ligand.get_center())

viewer = Viewer.render(scene, width=800, height=500)

viewer.save_image("screenshot.png")

print("Press Any Key to exit...", end="", flush=True)
_ = input()
