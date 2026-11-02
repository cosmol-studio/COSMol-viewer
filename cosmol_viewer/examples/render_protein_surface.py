import cosmolkit as ck
from cosmol_viewer import Cartoon, Scene, Surface, Viewer, select

mmcif_data = open("./examples/6fi1.cif", "r", encoding="utf-8").read()

protein = ck.Protein.from_mmcif(mmcif_data)

scene = Scene()
scene.set_scale(0.2)
scene.show(protein, select(protein).residues(), Cartoon().rainbow_residues())
scene.show(protein, select(protein).atoms(), Surface().color("#DCE8F2").opacity(0.9))
scene.recenter_on(protein)
scene.set_background_color("#021529")
scene.set_depth_cue(True)
scene.set_depth_cue_range(0.3,1.0)

viewer = Viewer.render(scene, width=800, height=500)

viewer.keep_alive()
