from pathlib import Path

import cosmolkit as ck
from cosmol_viewer import Cartoon, Scene, Viewer

mmcif_data = Path(__file__).with_name("6fi1.cif").read_text(encoding="utf-8")

prot = ck.Protein.from_mmcif(mmcif_data)

scene = Scene()
# scene.use_black_background()
scene.set_scale(0.25)
scene.recenter_on(prot)
scene.set("prot", prot, Cartoon().rainbow_residues())
scene.set_background_color("#021529")
scene.set_camera_view(azimuth=180, elevation=0, distance=32, fov=18)

scene.save_image("render_protein.png", width=1200, height=900)

viewer = Viewer.show(scene, width=800, height=500)

viewer.keep_alive()
