"""Draw four representations of one protein segment and assemble the PNG with Pillow."""

from io import BytesIO
from pathlib import Path

from cosmolkit import Element, Protein
from cosmol_viewer import Cartoon, Scene, SphereStyle, StickStyle, Viewer, select
from PIL import Image

package = Path(__file__).resolve().parents[1]
structure = package.parent / "docs-web/data/showcase/1db1-receptor.pdb"
protein = Protein.read(str(structure))
source = select(protein)
residues, atoms = source.residues(), source.atoms()

# Zero-based CK IDs of one segment of the 1DB1 receptor.
residue_ids = list(range(151, 174))
segment = residues.filter_ids(residue_ids)
heavy_atoms = atoms.residue_ids(residue_ids).difference(atoms.element(Element.H))
ca_ids = [
    atom.id
    for atom in protein.atoms
    if atom.residue.id in residue_ids and atom.name.as_str().strip() == "CA"
]

scene = Scene()
scene.recenter_on(protein)
scene.set_scale(0.222)
scene.set_camera_view(
    azimuth=-170, elevation=4, roll=8, distance=35, target=[0, 0.8, 0], fov=18
)
scene.set_background_color("#FFFFFF")
scene.set_lighting(ambient=0.55, diffuse=0.55, specular=0.14, color="#FFFFFF")
scene.set_multishadow()
scene.set_silhouettes(color="#141414", width=4)
white = Cartoon().color("#FFFFFF").roughness(0.85)
scene.set("context", protein, white)

blue = "#009ED6"
beads = SphereStyle().scale(0.62).color(blue)
panels = [
    (atoms.filter_ids(ca_ids[::2]), beads),
    (atoms.filter_ids(ca_ids), beads),
    (heavy_atoms, StickStyle().color(blue)),
    (segment, Cartoon().color(blue).roughness(0.85)),
]

width, height, gap = 680, 620, 16
figure = Image.new("RGB", (4 * width + 3 * gap, height), "white")
for index, (selection, style) in enumerate(panels):
    if index == 3:
        # The complementary selections reuse the same complete-chain Cartoon mesh.
        scene.set("context", residues.difference(segment), white)
    scene.set("detail", selection, style)
    png = scene.to_png(width=width, height=height)
    with Image.open(BytesIO(png)) as panel:
        figure.paste(panel, (index * (width + gap), 0))

output = package / "protein_representation_panels.png"
figure.save(output)
print(f"Saved {output}")

Viewer.show(scene, width=width, height=height).keep_alive()
