.. meta::
   :description: Build annotation geometry with COSMol Viewer spheres and sticks, positions, radii, colors, and materials.

Geometric Shapes
================

Spheres and sticks can annotate molecular scenes or build custom geometry.

.. code-block:: python

   from cosmol_viewer import Scene, Sphere, Stick

   marker = (
       Sphere(center=[0, 0, 0], radius=0.8)
       .color("#d1495b")
       .roughness(0.4)
   )
   axis = (
       Stick(start=[0, 0, 0], end=[3, 0, 0], thickness=0.12)
       .color("#00798c")
       .opacity(0.8)
   )

   scene = Scene()
   scene.add_shape(marker)
   scene.add_shape(axis)

``Sphere.set_center()`` and ``Sphere.set_radius()`` update sphere geometry.
``Stick.set_start()``, ``Stick.set_end()``, and ``Stick.set_thickness()`` update
stick geometry. Styling methods return the shape so calls can be chained.
