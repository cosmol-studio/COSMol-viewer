import cosmolkit as ck
from cosmol_viewer import BallAndStick, Scene, Viewer

base = (
    ck.Molecule.from_smiles("CC(=O)Nc1ccc(O)cc1")
    .with_hydrogens()
    .sanitize()
)

params = ck.EmbedParams.etkdg_v3()
params.random_seed = 0xF00D
params.num_threads = 1

cosmolkit_mol = base.with_3d_conformer(params)

mol = cosmolkit_mol

scene = Scene()
scene.set_scale(1.2)
scene.set_camera_view(azimuth=35, elevation=22, distance=28, fov=18)
scene.set("conformer", mol, BallAndStick().outline("#EEEEEE", 0.04))
scene.recenter_on(mol)

scene.save_image("render_3d_conformer_from_cosmolkit.png", width=1200, height=900)

viewer = Viewer.open(scene, width=800, height=500)

viewer.keep_alive()
