import cosmolkit as ck
from cosmol_viewer import BallAndStick, Scene, Viewer

cosmolkit_mol = ck.Molecule.from_smiles("COc1cc(C=Nn2c(SC)nnc2c3ccccc3)c(Br)cc1O")
cosmolkit_mol = cosmolkit_mol.with_hydrogens()

mol = cosmolkit_mol.with_2d_coordinates()

scene = Scene()
scene.set_scale(0.8)
scene.set("molecule", mol, BallAndStick().outline("#EEEEEE", 0.04))
scene.recenter_on(mol)

scene.save_image("render_molecule_from_cosmolkit.png", width=1200, height=900)

viewer = Viewer.show(scene, width=800, height=500)

viewer.keep_alive()
