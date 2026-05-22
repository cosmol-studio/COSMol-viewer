from cosmol_viewer import Molecule, Scene, Viewer

mol_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()

mol = Molecule.from_sdf(mol_data).centered().enable_outline(width=0.04)

scene = Scene()

scene.add_shape_with_id("molecule", mol)

viewer = Viewer.render(scene, width=800, height=500)

viewer.save_image("screenshot.png")

print("Press Any Key to exit...", end="", flush=True)
_ = input()
