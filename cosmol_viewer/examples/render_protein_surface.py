from cosmol_viewer import Protein, Scene, Viewer

mmcif_data = open("./examples/6fi1.cif", "r", encoding="utf-8").read()

protein = Protein.from_mmcif(mmcif_data).centered().rainbow_residues()

protein_surface = (
    Protein.from_mmcif(mmcif_data)
    .centered()
    .surface()
    .color("#DCE8F2")
    .opacity(0.9)
)

scene = Scene()
scene.set_scale(0.2)
scene.add_shape(protein)
scene.add_shape(protein_surface)
scene.set_background_color("#021529")
scene.set_depth_cue(True)
scene.set_depth_cue_range(0.3,1.0)

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
try:
    _ = input()
except EOFError:
    pass
