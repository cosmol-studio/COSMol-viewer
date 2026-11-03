.. meta::
   :description: Render your first molecular scene to PNG, display it in a notebook, and open an interactive COSMol Viewer canvas.

Quick Start
===========

Render an SDF molecule to a reproducible PNG:

.. code-block:: python

   from cosmolkit import Molecule
   from cosmol_viewer import Scene

   sdf = open("molecule.sdf", encoding="utf-8").read()
   molecule = Molecule.from_sdf(sdf)

   scene = Scene()
   scene.set("molecule", molecule)
   scene.recenter_on(molecule)
   scene.set_camera_view(
       azimuth=35,
       elevation=20,
       distance=32,
       fov=18,
   )
   scene.save_image("molecule.png", width=1200, height=900)

In a Jupyter or Colab notebook, display the same offscreen result inline:

.. code-block:: python

   scene.display(width=1200, height=900)

For an interactive view, let :class:`~cosmol_viewer.Viewer` select the native
desktop or notebook WebAssembly backend:

.. code-block:: python

   from cosmol_viewer import Viewer

   viewer = Viewer.open(scene, width=800, height=500)

``add(item)`` appends content; ``set(name, item)`` creates or replaces named
content. Shapes carry their own material. Molecules default to BallAndStick and
proteins to Cartoon; pass a style as the optional final argument to override it.
Scene methods mutate the scene in place. Native scripts can call
``viewer.keep_alive()`` to wait for the interactive window.
