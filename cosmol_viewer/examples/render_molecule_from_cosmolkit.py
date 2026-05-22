import cosmolkit
from cosmol_viewer import Molecule, Scene, Viewer

cosmolkit_mol = cosmolkit.Molecule.from_smiles("COc1cc(C=Nn2c(SC)nnc2c3ccccc3)c(Br)cc1O")
mol = (
    Molecule.from_cosmolkit(cosmolkit_mol)
    .centered()
    .enable_outline(width=0.04)
)

scene = Scene()
scene.set_scale(1.0)
scene.add_shape_with_id("molecule", mol)

viewer = Viewer.render(scene, width=800, height=500)
viewer.save_image("screenshot.png")

print("Press Any Key to exit...", end="", flush=True)
_ = input()
