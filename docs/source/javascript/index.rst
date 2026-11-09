JavaScript / TypeScript
=======================

.. meta::
   :description: Install COSMol Viewer, initialize its WebAssembly package, and open an interactive molecular scene in JavaScript or TypeScript.

Installation
------------

.. code-block:: shell

   npm install @cosmol-studio/cosmol-viewer@0.6.2 @cosmol-studio/cosmolkit@0.6.2

Use the package entry point so native constructors and scientific inputs are
registered. Importing Viewer automatically initializes both Viewer and the same
COSMolKit module; no explicit initialization calls are needed.

Place a canvas in your HTML at the desired location:

.. code-block:: html

   <canvas id="molecule-viewer" width="800" height="500"
           style="width:800px;height:500px"></canvas>

Run the JavaScript after this element exists. The example uses ``sdfText``
containing an SDF molecule; see :doc:`quickstart` for loading it from a file.

.. code-block:: javascript

   import * as cv from "@cosmol-studio/cosmol-viewer";
   import * as ck from "@cosmol-studio/cosmolkit";

   const molecule = ck.Molecule.fromSdf(sdfText);
   const scene = cv.Scene.new();
   scene.add(molecule); // default BallAndStick
   scene.recenterOn(molecule);
   const viewer = cv.Viewer.new("molecule-viewer");
   await viewer.render(scene);

Scene content
-------------

``add(item, style?)`` appends a display. ``set(name, item, style?)`` creates or
replaces named content. Both accept CK Molecule/Protein, source-bound Selection,
Sphere and Stick. A molecule defaults to BallAndStick and a protein to Cartoon.
Rust owns these defaults and validates style/entity compatibility.

``Viewer.new(canvasId)`` binds a viewer to your existing canvas. It does not
start rendering. You decide where the canvas is placed and how large it is.

Call ``await viewer.render(scene)`` to display a scene or
``await viewer.play(animation)`` to play an animation. Both reuse this viewer and
canvas; rendering a scene stops previous animation playback. For streaming
updates to an already active viewer, use ``viewer.update(scene)``.
``viewer.takeScreenshot()`` captures the current canvas.

Call ``viewer.close()`` to stop the renderer, then ``viewer.free()`` to release
the handle. Closing leaves your canvas in the page. A closed viewer cannot be
restarted; create a new viewer to reuse that canvas. Free other WASM handles
when they are no longer needed.

.. toctree::
   :maxdepth: 1

   quickstart
   workflows
   api
