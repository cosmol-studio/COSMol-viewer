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

Viewport Diagnostics
~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   viewer.show_fps(True)
   viewer.show_camera_parameters(True)
   viewer.camera_parameter_logging(False)

The overlays appear in the upper-left corner without intercepting dragging or
zooming. They are initially hidden and independent of terminal/browser-console
camera logging. Pass ``False`` to hide either overlay. These APIs also work in
Jupyter and Colab; JavaScript uses ``showFps``, ``showCameraParameters`` and
``cameraParameterLogging``.

FPS measures renderer repaint frequency averaged over half-second windows,
not animation frame rate or GPU timing. Neither overlay requests continuous
repainting. With FPS enabled, one second without activity triggers a one-shot
repaint to show zero FPS. That diagnostic repaint is not counted and does not
renew the idle timer; interaction or scene updates resume statistics. Overlays
are included in viewer screenshots, not ``Scene.to_png``.

Static native viewers are event-driven: interaction, window changes and incoming
commands request repainting, not an idle IPC timer. Animation playback and
automatic rotation still repaint continuously while active.

Camera Interaction
~~~~~~~~~~~~~~~~~~

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

For a native dynamic update loop, check the child-process lifecycle explicitly:

.. code-block:: python

   while viewer.is_open():
       # Modify the scene for the next frame.
       viewer.update(scene)

``is_open()`` returns ``False`` when the child exits, including an abnormal exit,
so the script can leave its loop and finish normally without ``keep_alive()``.
It does not listen for Enter and is unavailable in Jupyter or Colab. It does
not terminate the Python process: code after the loop can still run.

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
``Viewer.playNotebook(canvas_id, payload)``. Scenes, animations, and notebook
commands share the ``CMV2:R:<base64>`` (raw postcard) or ``CMV2:G:<base64>``
(gzip postcard) format. Serialized data below 1 KiB skips gzip; at or above
1 KiB it uses gzip level 1. Sender and receiver versions must match; old payload
formats are not accepted. Browser decoding defaults to a 64 MiB serialized-data
limit.
