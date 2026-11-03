Quick Start
===========

Load a molecule, compose a scene, open an interactive view, and export an image.
Choose Python or JavaScript above any code example to switch every example on
this page. The URL records your choice, including when reloading or sharing a link.

Read a Molecule
---------------

Use COSMolKit to read an SDF file containing molecular coordinates. The examples
below use a file named ``molecule.sdf``.

.. language-only:: python

   Follow the :doc:`installation` instructions first. Place the SDF file in
   your script's working directory.

.. language-only:: javascript

   Follow the :doc:`index` installation instructions first. Run this example
   in a browser application that resolves npm imports, and serve the SDF file
   at ``molecule.sdf`` relative to your page. Initialize both packages once.

.. code-example:: load-molecule

   .. code-block:: python

      from pathlib import Path
      from cosmolkit import Molecule
      from cosmol_viewer import Scene, Viewer

      sdf = Path("molecule.sdf").read_text(encoding="utf-8")
      molecule = Molecule.from_sdf(sdf)

   .. code-block:: javascript

      import * as cv from "@cosmol-studio/cosmol-viewer";
      import * as ck from "@cosmol-studio/cosmolkit";

      const response = await fetch("molecule.sdf");
      if (!response.ok) throw new Error(`SDF request failed: ${response.status}`);
      const molecule = ck.Molecule.fromSdf(await response.text());

Compose a Scene
---------------

Add the molecule and center the view on it. Molecules use ball-and-stick styling
by default; proteins use cartoons. A selection is only needed when displaying
a subset of atoms or residues.

.. code-example:: create-scene

   .. code-block:: python

      scene = Scene()
      scene.add(molecule)
      scene.recenter_on(molecule)

   .. code-block:: javascript

      const scene = cv.Scene.new();
      scene.add(molecule);
      scene.recenterOn(molecule);

Use ``add(item)`` to append content or ``set(name, item)`` to create or replace
named content. Both accept an optional style after the item.

Open an Interactive View
------------------------

Open the scene in an interactive view to rotate and zoom the molecule.

.. language-only:: python

   In a notebook, the viewer appears inline. In a desktop Python script it
   opens a window. Keep the returned viewer if you want to update the scene.

.. language-only:: javascript

   First place a canvas where the molecule should appear in your HTML. Set its
   size and give it an ID:

   .. code-block:: html

      <canvas id="molecule-viewer" width="800" height="500"
              style="width:800px;height:500px"></canvas>

   Run the following JavaScript after that element exists. ``Viewer.new`` binds
   a viewer to the canvas without rendering anything. Then ``viewer.render``
   displays the scene. Keep this viewer to change content, play animations,
   take screenshots or stop rendering. It does not create another canvas.

.. code-example:: open-viewer

   .. code-block:: python

      viewer = Viewer.open(scene, width=800, height=500)

   .. code-block:: javascript

      const viewer = cv.Viewer.new("molecule-viewer");
      await viewer.render(scene);

Export an Image
---------------

.. language-only:: python

   Export a PNG directly from the scene using offscreen rendering. Its size is
   independent of the interactive window; opening a viewer is optional for
   image export.

.. language-only:: javascript

   The browser API captures the current viewer canvas. It does not provide
   scene-level offscreen PNG export. This example downloads the screenshot
   using the canvas's current dimensions and view.

.. code-example:: export-image

   .. code-block:: python

      scene.save_image("molecule.png", width=1200, height=900)

   .. code-block:: javascript

      const png = await viewer.takeScreenshot();
      const url = URL.createObjectURL(new Blob([png], {type: "image/png"}));
      const download = document.createElement("a");
      download.href = url;
      download.download = "molecule.png";
      document.body.append(download);
      download.click();
      download.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);

.. language-only:: python

   In a desktop script, call ``viewer.keep_alive()`` after exporting if the
   interactive window should remain open. In a notebook, use ``scene.display()``
   to show a static image inline. See :doc:`rendering` for transparent PNGs and
   in-memory output.

.. language-only:: javascript

   When finished, call ``viewer.close()`` and release WASM handles with
   ``free()``. See :doc:`workflows` for scene updates, styles and animation.
