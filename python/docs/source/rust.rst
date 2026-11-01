.. meta::
   :description: Use the COSMol Viewer Rust facade to construct scenes and access the shared molecular rendering engine.

Rust API
========

The ``cosmol_viewer`` crate is the public Rust facade. It re-exports the shared
scene, animation, renderer, shape, and COSMolKit APIs used by the Python and
WebAssembly packages.

.. code-block:: rust

   use cosmol_viewer::{ImageRenderer, Scene, shapes::Molecule};

   fn main() -> Result<(), Box<dyn std::error::Error>> {
       let molecule = Molecule::from_sdf(include_str!("molecule.sdf"))?
           .centered()
           .enable_outline(0.04);

       let mut scene = Scene::new();
       scene.add_shape_with_id("molecule", molecule);
       ImageRenderer::save_png(&scene, "molecule.png", 1200, 900)?;
       Ok(())
   }

The complete generated Rust reference is published on
`docs.rs <https://docs.rs/cosmol_viewer/latest/cosmol_viewer/>`_. Rust examples
are maintained in the repository's ``cosmol_viewer/examples`` directory.
