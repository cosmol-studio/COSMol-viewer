.. meta::
   :description: Compose named shapes and configure camera angles, lighting, backgrounds, and depth cueing in COSMol Viewer scenes.

Scenes, Camera, and Lighting
============================

Scene Contents
--------------

Use IDs when a shape must later be replaced or removed:

.. code-block:: python

   from cosmol_viewer import Scene, Sphere

   scene = Scene()
   scene.add_shape_with_id("focus", Sphere([0, 0, 0], 1.0).color("#d1495b"))
   scene.replace_shape("focus", Sphere([1, 0, 0], 1.2).color("#00798c"))
   scene.remove_shape("focus")

``add_shape()`` is sufficient for static content that does not need an ID.
``recenter()`` changes the scene center and ``set_scale()`` applies a global
uniform scale.

Reproducible Camera
-------------------

The orbit camera is represented by azimuth, elevation, roll, distance, target,
and vertical field of view:

.. code-block:: python

   scene.set_camera_view(
       azimuth=180,
       elevation=0,
       roll=0,
       distance=32,
       target=[0, 0, 0],
       fov=18,
   )

Use ``rotate_camera()`` for relative changes. ``set_camera_distance()``,
``set_camera_target()``, and ``set_camera_fov()`` preserve the other camera
components.

For a native interactive viewer, camera parameter logging prints a compact
``set_camera_view(...)`` call whenever the camera moves:

.. code-block:: python

   from cosmol_viewer import Viewer

   viewer = Viewer.render(scene, width=800, height=500)
   viewer.set_camera_parameter_logging(True)

This logging feature is native-only and disabled by default.

Background and Depth Cueing
---------------------------

.. code-block:: python

   scene.set_background_color("#021529")
   scene.set_depth_cue(True)
   scene.set_depth_cue_range(0.3, 1.0)

Depth cueing fades distant fragments toward the scene background by default.
It is disabled initially; its default fractional range is ``0.5..1.0``.
Use ``set_depth_cue_color()`` to choose a different cue color. Transparent
interactive canvases are enabled with ``set_transparent_background(True)``;
static exports can independently request a transparent output background.

Lighting
--------

.. code-block:: python

   scene.set_lighting(
       ambient=0.55,
       diffuse=0.65,
       specular=1.0,
       intensity=1.0,
       color="#fff7f7",
   )

These values approximate the renderer defaults (the default light color is
stored as floating-point RGB ``[1.0, 0.97, 0.97]``).
``set_ambient_light()``,
``set_light_intensity()``, and ``set_light_color()`` update individual terms.
