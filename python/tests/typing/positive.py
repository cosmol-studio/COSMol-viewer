"""Fixed framework consumer samples; new business APIs need no changes here."""
from pathlib import Path
from typing_extensions import assert_type

from cosmol_viewer import Animation, Scene, Sphere, Stick


def use_typed_projections(path: Path) -> None:
    sphere = Sphere([0, 0, 0], 1)
    assert_type(sphere.color("#ff0000").opacity(0.5), Sphere)
    assert_type(sphere.color((255, 0, 0)), Sphere)
    stick = Stick([0, 0, 0], [1, 0, 0], 0.1)
    assert_type(stick.roughness(0.5).metallic(0.2), Stick)
    scene = Scene()
    scene.add(sphere)
    scene.set("bond", stick)
    scene.set("bond", sphere)
    scene.set_lighting(color=[255, 255, 255])
    scene.set_background_color("#ffffff")
    animation = Animation(0.1, -1, False)
    animation.add_frame(scene)
    assert_type(len(animation), int)
    assert_type(animation[-1], Scene)
    assert_type(animation.to_payload(), str)
    assert_type(scene.to_png(background="transparent"), bytes)
    scene.save_image(path, background=(255, 255, 255, 255))
    scene.save_image(str(path), background=(1.0, 1.0, 1.0, 0.5))
