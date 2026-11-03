"""Scientific consumers use the actual CK types from generated declarations."""
import cosmolkit as ck
import cosmol_viewer as cv
from typing_extensions import assert_type

def molecular(source: ck.Molecule) -> cv.Scene:
    assert_type(cv.select(source), cv.Selector)
    oxygen = cv.select(source).atoms().element(ck.Element.O)
    assert_type(oxygen, cv.Selection)
    assert_type(oxygen.ids, list[int])
    scene = cv.Scene()
    assert_type(scene.add(oxygen, cv.SphereStyle().scale(1.4)), None)
    scene.set("bonds", cv.select(source).bonds().aromatic(), cv.StickStyle())
    scene.add(source, cv.BallAndStick())
    scene.recenter_on(source)
    scene.add(source)
    scene.add(cv.Sphere([0, 0, 0], 1))
    scene.set("shape", cv.Stick([0, 0, 0], [1, 0, 0], 0.2))
    return scene

def protein(source: ck.Protein) -> cv.Scene:
    scene = cv.Scene()
    scene.add(source, cv.Cartoon().rainbow_residues())
    scene.add(source, cv.Surface().configure(grid_spacing=0.7))
    scene.add(source)
    scene.add(cv.Sphere([0, 0, 0], 1))
    scene.set("shape", cv.Stick([0, 0, 0], [1, 0, 0], 0.2))
    return scene
