from cosmol_viewer import Molecule, Scene, Viewer

mol_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()

mol = Molecule.from_sdf(mol_data).centered().enable_outline(width=0.04)

scene = Scene()

scene.add_shape_with_id("molecule", mol)
scene.set_camera_view(azimuth=35, elevation=20, distance=32, fov=18)

scene.save_image("rendered_molecule.png", width=1600, height=1000)
viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
try:
    _ = input()
except EOFError:
    pass
