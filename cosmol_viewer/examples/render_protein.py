from cosmol_viewer import Protein, Scene, Viewer

mmcif_data = open("./examples/6fi1.cif", "r", encoding="utf-8").read()

prot = Protein.from_mmcif(mmcif_data).rainbow_residues()

scene = Scene()
# scene.use_black_background()
scene.set_scale(0.2)
scene.recenter(prot.get_center())
scene.add_shape_with_id("prot", prot)
scene.set_background_color("#021529")

viewer = Viewer.render(scene, width=800, height=500)

print("Press Any Key to exit...", end="", flush=True)
_ = input()
