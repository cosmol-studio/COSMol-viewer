import cosmolkit as ck
from cosmol_viewer import Molecule, Scene, Viewer

base = (
    ck.Molecule.from_smiles("CC(=O)Nc1ccc(O)cc1")
    .with_hydrogens()
    .sanitize()
)

params = ck.EmbedParameters.etkdg_v3()
params.random_seed = 0xF00D
params.num_threads = 1

cosmolkit_mol = base.with_3d_conformer(params)

mol = (
    Molecule.from_cosmolkit(cosmolkit_mol)
    .centered()
    .enable_outline(width=0.04)
)

scene = Scene()
scene.set_scale(1.2)
scene.set_camera_view(azimuth=35, elevation=22, distance=28, fov=18)
scene.add_shape_with_id("conformer", mol)

scene.save_image("render_3d_conformer_from_cosmolkit.png", width=1200, height=900)

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
try:
    _ = input()
except EOFError:
    pass
