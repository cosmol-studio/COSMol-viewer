import math

from cosmol_viewer import Animation, Scene, Sphere, Viewer

ids = ["a", "b", "c", "d", "e", "f"]
interval = 0.05
duration = 10.0
num_frames = int(duration / interval)

animation = Animation(interval, -1, True)

for frame_idx in range(num_frames):
    t = frame_idx * interval / duration * math.pi * 4.0
    scene = Scene()
    scene.set_scale(2.0)

    for i, id in enumerate(ids):
        phase = i * math.pi / 3.0
        theta = t + phase

        # Elliptical trajectory
        x = 1.5 * math.cos(theta)
        y = 0.8 * math.sin(theta)
        z = 0.5 * math.sin(theta * 1.5)

        # Radius pulsation
        radius = 0.3 + 0.15 * math.sin(theta * 1.5)

        # Dynamic color
        r = 0.5 + 0.5 * math.sin(theta)
        g = 0.5 + 0.5 * math.cos(theta)
        b = 1.0 - r

        sphere = Sphere([x, y, z], radius).color((int(r* 255), int(g* 255), int(b* 255)))
        scene.add_shape_with_id(id, sphere)

    animation.add_frame(scene)

# One-time submission: interval of 0.02 seconds
Viewer.play(animation, width=800.0, height=500.0)

print("Press Any Key to exit...", end="", flush=True)
_ = input()
