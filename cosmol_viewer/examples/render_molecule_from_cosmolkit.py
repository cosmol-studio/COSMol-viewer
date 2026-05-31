import cosmolkit as ck
from cosmol_viewer import Molecule, Scene, Viewer

cosmolkit_mol = ck.Molecule.from_smiles("COc1cc(C=Nn2c(SC)nnc2c3ccccc3)c(Br)cc1O")
cosmolkit_mol = cosmolkit_mol.with_hydrogens()

mol = (
    Molecule.from_cosmolkit(cosmolkit_mol)
    .centered()
    .enable_outline(width=0.04)
)

scene = Scene()
scene.set_scale(0.8)
scene.add_shape_with_id("molecule", mol)

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
try:
    _ = input()
except EOFError:
    pass
