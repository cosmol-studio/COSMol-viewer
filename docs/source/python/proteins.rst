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

.. figure:: https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_protein.png
   :alt: Protein ribbon rendering with independently colored chains
   :width: 100%

   Protein cartoon geometry rendered by COSMol Viewer.

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
