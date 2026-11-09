from pathlib import Path

import cosmolkit as ck
from cosmol_viewer import Cartoon, Scene, Surface, Viewer

mmcif_data = Path(__file__).with_name("6fi1.cif").read_text(encoding="utf-8")

protein = ck.Protein.from_mmcif(mmcif_data)

scene = Scene()
scene.set_scale(0.2)
scene.add(protein, Cartoon().rainbow_residues())
scene.add(protein, Surface().color("#DCE8F2").opacity(0.9))
scene.recenter_on(protein)
scene.set_background_color("#021529")
scene.set_depth_cue(True)
scene.set_depth_cue_range(0.3,1.0)

scene.save_image("render_protein_surface.png", width=1200, height=900)

viewer = Viewer.show(scene, width=800, height=500)

viewer.keep_alive()
