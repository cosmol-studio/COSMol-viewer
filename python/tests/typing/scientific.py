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
    assert_type(scene.show(source, oxygen, cv.SphereStyle().scale(1.4)), None)
    scene.show_with_id("bonds", source, cv.select(source).bonds().aromatic(), cv.StickStyle())
    scene.recenter_on(source)
    return scene

def protein(source: ck.Protein) -> cv.Scene:
    scene = cv.Scene()
    scene.show(source, cv.select(source).residues(), cv.Cartoon().rainbow_residues())
    scene.show(source, cv.select(source).atoms(), cv.Surface().configure(grid_spacing=0.7))
    return scene
