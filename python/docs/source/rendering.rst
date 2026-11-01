.. meta::
   :description: Export opaque or transparent PNG images, render in memory, and display molecular scenes in notebooks and headless environments.

Static and Offscreen Rendering
==============================

Save a PNG
----------

Static output belongs to :class:`~cosmol_viewer.Scene` and does not require a
visible :class:`~cosmol_viewer.Viewer`:

.. code-block:: python

   scene.save_image("figure.png", width=1600, height=1000)

The output uses the scene background unless ``background`` overrides it:

.. code-block:: python

   scene.save_image("white.png", background="#ffffff")
   scene.save_image("transparent.png", background="transparent")

PNG Bytes
---------

Use ``to_png()`` for web responses, archives, and in-memory workflows:

.. code-block:: python

   png = scene.to_png(width=1200, height=800, background="transparent")
   assert png.startswith(b"\x89PNG")

Notebook Display
----------------

``display()`` renders the same static PNG and passes it to ``IPython.display``:

.. code-block:: python

   scene.display(width=1200, height=800)

It is intentionally different from ``Viewer.render()``: ``display()`` produces
a static image, while ``Viewer.render()`` creates an interactive canvas.

Headless Environments
---------------------

On Linux without ``DISPLAY`` or Wayland, the renderer uses headless EGL. Google
Colab automatically renders in an isolated software-GL child process so a
native driver failure cannot restart the notebook kernel. No Colab-specific
environment setup is required.

The following environment variables remain available for diagnostics and
explicit overrides:

``COSMOL_VIEWER_RENDER_ISOLATED``
   Force or disable isolated Python image rendering.

``COSMOL_VIEWER_OFFSCREEN_SAMPLES``
   Override the offscreen sample count. Software renderers default to one
   sample; hardware renderers default to four.

``LIBGL_ALWAYS_SOFTWARE`` and ``GALLIUM_DRIVER``
   Select Mesa software rendering behavior on Linux. Colab supplies safe
   defaults inside its child renderer without modifying the notebook process.
