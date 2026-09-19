.. meta::
   :description: COSMol Viewer Python documentation for molecular scenes, protein ribbons and surfaces, interactive viewers, animations, and PNG export.

COSMol Viewer
==============

COSMol Viewer is a Python and Rust molecular visualization library backed by a
shared Rust renderer. It supports static PNG export, native desktop viewers,
interactive Jupyter and Google Colab canvases, molecular animations, protein
cartoons, and molecular surfaces.

The Python API is organized around three concepts:

* A :class:`~cosmol_viewer.Scene` owns shapes and rendering settings.
* Shapes such as :class:`~cosmol_viewer.Molecule`,
  :class:`~cosmol_viewer.Protein`, :class:`~cosmol_viewer.Sphere`, and
  :class:`~cosmol_viewer.Stick` describe visible geometry.
* :class:`~cosmol_viewer.Viewer` creates an interactive native or notebook
  view. Static output is produced directly by the scene.

.. raw:: html

   <div style="display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;margin:1.5rem 0;">
     <img src="https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_molecule_stick.png" alt="Stick molecule rendering" style="width:100%;" />
     <img src="https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_3d_conformer_from_cosmolkit.png" alt="COSMolKit conformer rendering" style="width:100%;" />
     <img src="https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/render_protein.png" alt="Protein cartoon rendering" style="width:100%;" />
     <img src="https://pub-0588ab5197fd48f28b5c91f067adf8f4.r2.dev/image/protein_ligand.png" alt="Protein and ligand rendering" style="width:100%;" />
   </div>

.. toctree::
   :maxdepth: 2
   :caption: User Guide

   installation
   quickstart
   scenes
   molecules
   proteins
   geometry
   rendering
   viewer
   rust
   api
