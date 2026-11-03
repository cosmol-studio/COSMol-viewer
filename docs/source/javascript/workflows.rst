JavaScript Workflows
====================

.. meta::
   :description: Compose molecular and protein scenes, reuse imported sources, select subsets, update shapes, animate scenes and capture browser images with COSMol Viewer.

Protein and molecular styles
----------------------------

.. code-block:: javascript

   const protein = ck.Protein.fromMmcif(cifText);
   const scene = cv.Scene.new();
   scene.set("protein", protein); // default Cartoon
   const surface = cv.Style.surface().opacity(0.5);
   scene.add(protein, surface);
   scene.recenterOn(protein);

The same live CK object is imported once. ``add``, ``set``, ``select`` and
``recenterOn`` reuse the native snapshot. If scientific data changes while that
snapshot is retained, pass a new CK object to import the edited value. Styles and
camera operations do not mutate the scientific source.

Selections
----------

.. code-block:: javascript

   const selector = cv.select(protein);
   const atoms = selector.atoms();
   const oxygen = ck.Element.fromSymbol("O");
   const selected = atoms.element(oxygen);
   const highlight = cv.Style.sphere().scale(1.4);
   scene.set("oxygen", selected, highlight);

Selections already retain their source; never pass it again. Molecular atom
selections default to BallAndStick, molecular bonds to Stick, protein atoms to
Sphere, and residues to Cartoon. Molecular bond selections support ``aromatic()``.

Shapes and dynamic updates
--------------------------

.. code-block:: javascript

   const marker = cv.Sphere.new(new Float32Array([0, 0, 0]), 0.5);
   scene.set("marker", marker);
   const moved = cv.Sphere.new(new Float32Array([1, 0, 0]), 0.5);
   scene.set("marker", moved);
   viewer.update(scene);

Shapes carry their own material. ``color`` and ``opacity`` return styled copies;
passing an additional scientific Style to a positioned shape is an error.
Scene insertion snapshots shape values, so later handle disposal is safe.

Animation
---------

Create a canvas for playback and bind a viewer before starting the animation:

.. code-block:: html

   <canvas id="animation-viewer" width="800" height="500"
           style="width:800px;height:500px"></canvas>

.. code-block:: javascript

   const animation = cv.Animation.new(0.05, -1n, false);
   animation.addFrame(scene);
   scene.setScale(1.2);
   animation.addFrame(scene);
   const playing = cv.Viewer.new("animation-viewer");
   await playing.play(animation);

Frames are snapshots; scenes remain reusable. Loop counts use ``bigint``.
Positions use ``Float32Array`` and RGB colors use ``Uint8Array``.

Images
------

.. code-block:: javascript

   const png = await viewer.takeScreenshot();
   const image = new Blob([png], {type: "image/png"});

Browser capture returns PNG bytes from the active canvas. Python and Rust also
provide scene-level offscreen image export, independent of window dimensions.
