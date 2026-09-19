.. meta::
   :description: Use native and notebook viewers, update scenes, control interaction, and play or export molecular animations.

Interactive Viewers and Animation
=================================

Runtime Selection
-----------------

``Viewer.render()`` selects its backend from the Python runtime:

* Jupyter and Google Colab use an inline WebAssembly canvas.
* Plain Python scripts and terminal IPython use a native GUI window.

.. code-block:: python

   from cosmol_viewer import Viewer

   print(Viewer.get_environment())
   viewer = Viewer.render(scene, width=800, height=500)

The returned viewer must remain referenced while it is being updated.

Interaction Controls
--------------------

Scene settings can keep drag rotation while disabling zoom, or automatically
orbit the molecule around the current camera-relative horizontal axis:

.. code-block:: python

   scene.set_zoom_disabled(True)
   scene.set_auto_rotate(True, speed=20.0)

Streaming Updates
-----------------

Use IDs to replace content in a scene, then send the new scene to an existing
viewer:

.. code-block:: python

   scene.replace_shape("molecule", next_molecule)
   viewer.update(scene)

``update()`` is intended for live or streaming data where frames are not known
in advance.

Animation Playback
------------------

Use :class:`~cosmol_viewer.Animation` when all frames are available before
playback:

.. code-block:: python

   from cosmol_viewer import Animation, Scene, Viewer

   animation = Animation(interval=0.05, loops=-1, interpolate=False)
   for molecule in molecules:
       frame = Scene()
       frame.add_shape(molecule)
       animation.add_frame(frame)

   Viewer.play(animation, width=800, height=500)

``interval`` is measured in seconds. ``loops=-1`` repeats indefinitely.
``interpolate=True`` interpolates compatible scene frames. A static scene can
be attached with ``set_static_scene()`` for geometry shared by every frame.
.. note::

   Development builds also allow changes after construction with
   ``animation.set_interval(0.02)``, ``animation.set_loops(3)``, and
   ``animation.set_interpolate(True)``. These setters are not part of the
   published 0.3.0 reference used to build this documentation.

Animation Payloads
------------------

An animation can be serialized for later browser playback:

.. code-block:: python

   from pathlib import Path

   Path("trajectory.cmv").write_text(animation.to_payload(), encoding="utf-8")

The payload contains the animation settings and frames. Browser playback uses
the existing WebAssembly viewer. The JavaScript documentation track is reserved
for the future standalone scene-building API.
