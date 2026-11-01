.. meta::
   :description: Render your first molecular scene to PNG, display it in a notebook, and open an interactive COSMol Viewer canvas.

Quick Start
===========

Render an SDF molecule to a reproducible PNG:

.. code-block:: python

   from cosmol_viewer import Molecule, Scene

   sdf = open("molecule.sdf", encoding="utf-8").read()
   molecule = Molecule.from_sdf(sdf).centered().enable_outline(width=0.04)

   scene = Scene()
   scene.add_shape_with_id("molecule", molecule)
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

   viewer = Viewer.render(scene, width=800, height=500)

Shape methods such as ``centered()``, ``stick()``, ``color()``, and
``opacity()`` update the Python shape and return it, allowing method chaining.
Scene methods mutate the scene in place.
