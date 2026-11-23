.. meta::
   :description: Load SDF molecules and COSMolKit conformers, choose molecular representations, and configure colors, materials, and outlines.

Molecules
=========

SDF Input
---------

Load scientific data with COSMolKit, then pass the molecule directly to a scene:

.. code-block:: python

   from cosmolkit import Molecule
   from cosmol_viewer import Scene, BallAndStick, StickStyle, SphereStyle

   sdf = open("ligand.sdf", encoding="utf-8").read()
   molecule = Molecule.from_sdf(sdf)
   scene = Scene()
   scene.add(molecule)
   scene.recenter_on(molecule)

COSMolKit Integration
---------------------

The viewer imports each live ``cosmolkit.Molecule`` once through its transfer
binary. Scene operations and selections share that immutable native snapshot.
Stored coordinates are required: generate coordinates explicitly in COSMolKit
when needed. Multiple 3D conformers require an explicit style ``conformer(id)``.
To display changes while an old import is still live, pass a new CK object.

Representations
---------------

.. code-block:: python

   scene.set("molecule", molecule)  # default BallAndStick
   scene.set("molecule", molecule, StickStyle())
   scene.set("molecule", molecule, SphereStyle())

The stick representation retains double and triple bond separation and renders
aromatic bonds as a single stick with an inner aromatic line.

.. figure:: https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_molecule_stick.png
   :alt: Molecular stick representation with atom colors and outlines
   :width: 100%

   A molecular stick rendering produced by COSMol Viewer.

Materials and Silhouettes
-------------------------

Scientific styles support ``color()``, ``opacity()``, ``roughness()``, and
``metallic()``. Set silhouettes on an individual display with a width in
output pixels.

.. code-block:: python

   style = BallAndStick().roughness(0.55).metallic(0.0).silhouettes("#101010", 2)
   scene.set("molecule", molecule, style)

Use ``scene.set_silhouettes()`` to set the default for the whole scene. Display
settings take precedence over the scene default. A display width of zero
disables its silhouettes.

``roughness`` and ``metallic`` use values from 0 to 1. Scientific data stays in
COSMolKit; display styles do not change molecule coordinates or chemistry.

Molecular colors
----------------

.. include:: ../shared/molecular_colors.inc

The code below uses three positioned CK molecules, ``molecule``, ``first``,
and ``second``. The last two share atom IDs for reuse of one highlight
scheme. Python accepts hex strings or RGB byte sequences.

.. code-block:: python

   from cosmol_viewer import MoleculeColors, Scene, StickStyle

   scene = Scene()
   palette = MoleculeColors(elements={"C": "#5CCBE5"})
   scene.set_molecule_palette(palette)
   scene.set("reference", molecule, StickStyle())

   shared = palette.with_atom_colors({0: "#FFD166"})
   highlighted = StickStyle().colors(shared)
   scene.set("first", first, highlighted)
   scene.set("second", second, highlighted)

   # Give only the second display a different atom color.
   pink = palette.with_atom_colors({0: "#F283CB"})
   scene.set("second", second, StickStyle().colors(pink))

   defaults = MoleculeColors.default()
   scene.set("second", second, StickStyle().colors(defaults))
   scene.set_molecule_palette(defaults)

Run the `Python example <https://github.com/cosmol-studio/COSMol-viewer/blob/main/cosmol_viewer/examples/molecule_colors.py>`_
from the repository root.

.. code-block:: shell

   python cosmol_viewer/examples/molecule_colors.py

The example opens an interactive viewer and exports ``molecule_colors.png``
and ``molecule_colors_builtin.png`` offscreen into ``cosmol_viewer/``.
In the custom image, top left uses the scene palette, top right and bottom
left share yellow highlights, and bottom right uses a separate pink scheme.
