.. meta::
   :description: Use the COSMol Viewer Rust facade to construct scenes and access the shared molecular rendering engine.

Rust API
========

The ``cosmol_viewer`` crate is the public Rust facade. It re-exports the shared
scene, animation, renderer, shape, and COSMolKit APIs used by the Python and
WebAssembly packages.

Install the public crate with ``cargo add cosmol_viewer``.

.. code-block:: rust

   use cosmol_viewer::{ImageRenderer, Scene};
   use cosmol_viewer::cosmolkit::Molecule;

   fn main() -> Result<(), Box<dyn std::error::Error>> {
       let molecule = Molecule::from_sdf(include_str!("molecule.sdf"))?;

       let mut scene = Scene::new();
       scene.set("molecule", &molecule)?;
       scene.recenter_on(&molecule)?;
       ImageRenderer::save_png(&scene, "molecule.png", 1200, 900)?;
       Ok(())
   }

The complete generated Rust reference is published on
`docs.rs <https://docs.rs/cosmol_viewer/latest/cosmol_viewer/>`_. Rust examples
are maintained in the repository's ``cosmol_viewer/examples`` directory.

``scene.add(&molecule)?`` uses the default style. To choose one explicitly, pass
an object/style tuple: ``scene.add((&protein, Surface::default()))?``.
Positioned shapes use the same methods: ``scene.set("marker", sphere)?``.
