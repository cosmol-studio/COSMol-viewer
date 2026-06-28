from cosmol_viewer import Molecule, Scene, Viewer

mol_data = open("./examples/6fi1_ligand.sdf", "r", encoding="utf-8").read()

mol = Molecule.from_sdf(mol_data).centered().stick().enable_outline(width=0.04)

scene = Scene()
scene.set_scale(1.2)

scene.add_shape_with_id("molecule", mol)
scene.set_camera_view(azimuth=-55, elevation=20, distance=32, fov=18)

scene.save_image("render_molecule_stick.png", width=1200, height=900)

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
try:
    _ = input()
except EOFError:
    pass
