import math
import time

from cosmol_viewer import Scene, Sphere, Viewer

scene = Scene()

ids = ["a", "b", "c", "d", "e", "f"]
for id in ids:
    sphere = Sphere([0.0, 0.0, 0.0], 0.4).color("#FFFFFF")
    scene.add_shape_with_id(id, sphere)

scene.set_scale(2.0)

viewer = Viewer.render(scene, width=800.0, height=500.0)

# === time-driven animation ===
start_time = time.perf_counter()
angular_speed = 0.4 * math.pi

frame_interval = 0.005  # seconds (5 ms)

while True:
    elapsed = time.perf_counter() - start_time
    t = elapsed * angular_speed

    for i, id in enumerate(ids):
        phase = i * math.pi / 3.0
        theta = t + phase

        x = 1.5 * math.cos(theta)
        y = 0.8 * math.sin(theta)
        z = 0.5 * math.sin(theta * 2.0)

        radius = 0.3 + 0.15 * math.sin(theta * 1.5)

        r = 0.5 + 0.5 * math.sin(theta)
        g = 0.5 + 0.5 * math.cos(theta)
        b = 1.0 - r

        sphere = Sphere([x, y, z], radius).color((int(r* 255), int(g* 255), int(b* 255)))
        scene.replace_shape(id, sphere)

    viewer.update(scene)

    time.sleep(frame_interval)
