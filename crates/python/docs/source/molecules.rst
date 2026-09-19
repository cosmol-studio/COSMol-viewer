.. meta::
   :description: Load SDF molecules and COSMolKit conformers, choose molecular representations, and configure colors, materials, and outlines.

Molecules
=========

SDF Input
---------

The viewer accepts an SDF string and uses COSMolKit for parsing:

.. code-block:: python

   from cosmol_viewer import Molecule

   sdf = open("ligand.sdf", encoding="utf-8").read()
   molecule = Molecule.from_sdf(sdf).centered()

COSMolKit Integration
---------------------

Convert an existing Python ``cosmolkit.Molecule`` without crossing through a
temporary SDF file. Stored 3D coordinates are preferred; stored 2D coordinates
are used next, and 2D coordinates are generated if neither is available.

.. code-block:: python

   import cosmolkit as ck
   from cosmol_viewer import Molecule

   source = (
       ck.Molecule.from_smiles("CC(=O)Nc1ccc(O)cc1")
       .with_hydrogens()
       .with_3d_conformer()
   )
   molecule = Molecule.from_cosmolkit(source).centered()

Representations
---------------

.. code-block:: python

   ball_and_stick = Molecule.from_sdf(sdf).ball_and_stick()
   sticks = Molecule.from_sdf(sdf).stick()
   space_filling = Molecule.from_sdf(sdf).sphere()

The stick representation retains double and triple bond separation and renders
aromatic bonds as a single stick with an inner aromatic line.

.. figure:: https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_molecule_stick.png
   :alt: Molecular stick representation with atom colors and outlines
   :width: 100%

   A molecular stick rendering produced by COSMol Viewer.

Materials and Outlines
----------------------

All shapes support ``color()``, ``opacity()``, ``roughness()``, and
``metallic()``. Molecules additionally support an imposter-based outline:

.. code-block:: python

   molecule = (
       Molecule.from_sdf(sdf)
       .centered()
       .roughness(0.55)
       .metallic(0.0)
       .enable_outline(color="#101010", width=0.04)
   )

``roughness`` and ``metallic`` use values from 0 to 1. Use
``disable_outline()`` or ``set_outline(False)`` to remove the outline.
