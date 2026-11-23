.. meta::
   :description: Read PDB and mmCIF structures, color protein ribbons, and configure solvent-excluded and solvent-accessible molecular surfaces.

Proteins and Surfaces
=====================

Protein Input
-------------

Create a COSMolKit protein from PDB or mmCIF text:

.. code-block:: python

   from cosmolkit import Protein
   from cosmol_viewer import Scene, Cartoon, Surface

   cif = open("protein.cif", encoding="utf-8").read()
   protein = Protein.from_mmcif(cif)

   pdb = open("protein.pdb", encoding="utf-8").read()
   protein_from_pdb = Protein.from_pdb(pdb)

COSMolKit parses the structure and the viewer assigns secondary structure from
backbone geometry before generating a ChimeraX-style cartoon ribbon.

Cartoon Ribbons
---------------

.. code-block:: python

   scene = Scene()
   scene.add(protein)  # default Cartoon
   scene.set("rainbow", protein, Cartoon().rainbow_residues())

``rainbow_residues()`` colors every biopolymer chain independently from its
first to last rendered residue. Use ``color()`` for a uniform ribbon color.

Cartoon geometry is computed from complete chains. Residue selections partition
that geometry without introducing new spline endpoints or caps, so complementary
selections meet at the same boundaries. Live displays with identical protein
inputs share the geometry computation across colors and opacity settings.
Each display still copies its selected mesh patches and assigns its material;
this does not share GPU buffers. The cached geometry is released with its last
display, and changed protein inputs require new geometry.

.. figure:: https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_protein.png
   :alt: Protein ribbon rendering with independently colored chains
   :width: 100%

   Protein cartoon geometry rendered by COSMol Viewer.

Scene Silhouettes
-----------------

Enable depth-based silhouettes around all visible geometry, including overlapping
ribbons, molecular surfaces, atoms, bonds and positioned shapes. Line width is
in output pixels (1 to 16). The depth jump is a fraction of the scene's depth
range.

.. code-block:: python

   scene.set_silhouettes(color="#505050", width=1, depth_jump=0.03)
   scene.save_image("outlined-protein.png", width=1600, height=1200)

Use ``Cartoon().silhouettes("#505050", 2)`` as the style for a single protein
display. Surface styles support the same method.

Run the ``scene_silhouettes.py`` example to export the same view with silhouettes
off, on, and beneath a transparent surface.

.. code-block:: sh

   python cosmol_viewer/examples/scene_silhouettes.py

The example opens an interactive window after exporting the PNGs.

Atoms and Bonds
---------------

Pass a protein directly to ``BallAndStick`` or ``StickStyle`` to display its
atoms and chemical bonds, using the same rendering geometry as molecules:

.. code-block:: python

   from cosmol_viewer import StickStyle, select

   scene = Scene()
   scene.add(protein, StickStyle())

To color side chains separately, partition the atoms into complementary selections:

.. code-block:: python

   atoms = select(protein).atoms()
   highlighted = atoms.residue_ids([10, 11, 12, 13, 14]).sidechain()
   rest = atoms.difference(highlighted)
   scene = Scene()
   scene.set("protein", rest, StickStyle())
   scene.set("sidechains", highlighted, StickStyle().color("#FF4444"))
   scene.recenter_on(protein)

``residue_ids`` accepts zero-based CK residue IDs, not PDB sequence numbers.
``sidechain`` selects heavy atoms excluding backbone names N, CA, C, O and OXT;
it excludes all hydrogens. ``filter_ids`` intersects a selection with explicit
entity IDs. ``difference`` requires the same source object and entity kind.
These operations return new selections and retain the original source.

Protein atom selections with BallAndStick or Stick partition each complete bond
at its midpoint. Complementary selections with the same geometry settings meet
there, even when their colors differ. Use a fresh scene as above, rather than
adding a second overlapping whole-protein display.

The first BallAndStick or Stick display converts the protein's chemical graph
through COSMolKit in Rust. Conversion disables sanitization and hydrogen removal,
preserves alternate atoms and coordinates, and uses CK proximity bonding to
supplement the source connectivity. It does not optimize coordinates. Live
displays reuse the converted graph; dropping the last display releases it.
Sphere and Cartoon do not require this conversion. The default for a whole
protein remains Cartoon; an atom selection still defaults to Sphere.

Molecular Surfaces
------------------

``Surface()`` configures a molecular surface. Set ``solvent_accessible=True``
for a solvent-accessible representation.

.. code-block:: python

   surface = Surface().configure(
       probe_radius=1.4,
       grid_spacing=0.5,
       solvent_accessible=False,
       sharp_boundaries=True,
   ).color("#dce8f2").opacity(0.9)

Smaller grid spacing produces a denser mesh and increases surface generation
cost. ``sharp_boundaries=True`` subdivides atom patches so shared boundaries
are exact.

Combining Representations
-------------------------

Reuse the same ``Protein`` for a ribbon and surface. The binding imports it
only once while the scene or a selection retains it:

.. code-block:: python

   from cosmol_viewer import Scene

   scene = Scene()
   scene.add(protein)
   scene.add(protein, surface)
   scene.set_depth_cue(True)
