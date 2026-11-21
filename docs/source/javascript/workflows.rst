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

Replacing or removing named content releases the old imported source once no
other content in that scene uses it. Selections, cloned scenes and merged content
retain their own references. Failed replacements preserve the old content and
ownership. ``recenterOn`` stores only the center. After all owners release an
import, passing the CK object again imports its current data.

Molecular colors
----------------

.. include:: ../shared/molecular_colors.inc

The code below uses three positioned CK molecules, ``molecule``, ``first``,
and ``second``. The last two share atom IDs for reuse of one highlight
scheme. JavaScript accepts RGB colors as ``Uint8Array`` values.

.. code-block:: javascript

   const coloredScene = cv.Scene.new();
   const palette = cv.MoleculeColors.new({C: new Uint8Array([92, 203, 229])});
   coloredScene.setMoleculePalette(palette);
   const sticks = cv.Style.stick();
   coloredScene.set("reference", molecule, sticks);

   const shared = palette.withAtomColors({0: new Uint8Array([255, 209, 102])});
   const highlighted = sticks.colors(shared);
   coloredScene.set("first", first, highlighted);
   coloredScene.set("second", second, highlighted);

   // Give only the second display a different atom color.
   const pink = palette.withAtomColors({0: new Uint8Array([242, 131, 203])});
   const pinkSticks = sticks.colors(pink);
   coloredScene.set("second", second, pinkSticks);

   const defaults = cv.MoleculeColors.default();
   const builtinSticks = sticks.colors(defaults);
   coloredScene.set("second", second, builtinSticks);
   coloredScene.setMoleculePalette(defaults);
   for (const value of [builtinSticks, defaults, pinkSticks, pink, highlighted, shared, sticks, palette]) {
       value.free();
   }

Run the `browser example <https://github.com/cosmol-studio/COSMol-viewer/tree/main/wasm/examples/browser>`_
and select **Molecular colors**. Top left uses the scene palette, top right and
bottom left share yellow highlights, and bottom right uses a separate pink
scheme. Use **Restore built-in colors** to compare with the original appearance.

Selections
----------

.. code-block:: javascript

   const selector = cv.select(protein);
   const atoms = selector.atoms();
   const oxygen = ck.Element.fromSymbol("O");
   const selected = atoms.element(oxygen);
   const highlight = cv.Style.sphere().scale(1.4);
   scene.set("oxygen", selected, highlight);

Selections already retain their source; never pass it again. Atom selections
default to Sphere regardless of their source, bond selections to Stick, and
residue selections to Cartoon. Whole Molecule objects default to BallAndStick
and whole Protein objects to Cartoon. Molecular bond selections support ``aromatic()``.

Protein atoms and bonds
-----------------------

Proteins also accept ``Style.ballAndStick()`` and ``Style.stick()`` directly:

.. code-block:: javascript

   const molecularScene = cv.Scene.new();
   const sticks = cv.Style.stick();
   molecularScene.add(protein, sticks);
   sticks.free();

For independently colored side chains, build complementary atom selections:

.. code-block:: javascript

   const source = cv.select(protein);
   const allAtoms = source.atoms();
   const selectedResidues = allAtoms.residueIds([10, 11, 12, 13, 14]);
   const sidechains = selectedResidues.sidechain();
   const remaining = allAtoms.difference(sidechains);
   const baseStyle = cv.Style.stick();
   const red = baseStyle.color(new Uint8Array([255, 68, 68]));
   const coloredScene = cv.Scene.new();
   coloredScene.set("protein", remaining, baseStyle);
   coloredScene.set("sidechains", sidechains, red);
   coloredScene.recenterOn(protein);
   // An existing <canvas id="protein-viewer"> is required.
   const proteinViewer = cv.Viewer.new("protein-viewer");
   await proteinViewer.render(coloredScene);
   for (const value of [red, baseStyle, remaining, sidechains,
                        selectedResidues, allAtoms, source]) value.free();
   // Keep coloredScene for updates; dispose it and proteinViewer when finished.

``residueIds`` accepts zero-based CK residue IDs, not PDB sequence numbers.
``sidechain`` selects heavy atoms excluding H and backbone names N, CA, C, O,
OXT. ``filterIds([id, ...])`` intersects explicit entity IDs; ``difference``
requires the same source object and entity kind. Selections remain reusable.

Protein BallAndStick/Stick selections split complete bonds at their midpoints,
so complementary selections meet without missing connections. CK chemical graph
conversion runs in Rust once while its displays are retained, preserving atoms
and coordinates with sanitization and hydrogen removal disabled. CK proximity
bonding supplements source connectivity; Viewer adds no separate bond inference.
Whole proteins still default to Cartoon and atom selections to Sphere.

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
