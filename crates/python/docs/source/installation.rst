.. meta::
   :description: Install COSMol Viewer from PyPI, verify the installation, and choose a desktop, notebook, or headless rendering environment.

Installation
============

Python
------

Install the released Python package from PyPI:

.. code-block:: bash

   pip install cosmol-viewer

The wheel contains the native Python extension and the WebAssembly assets used
by interactive Jupyter and Google Colab viewers. A separate JavaScript install
is not required for Python use.

Rust
----

Add the Rust facade crate:

.. code-block:: bash

   cargo add cosmol_viewer

Source Checkout
---------------

For Python binding development, activate the project environment and install
the extension in editable mode from the repository root:

.. code-block:: bash

   uv venv .venv
   uv pip install --python .venv/bin/python "maturin>=1.7,<2.0" "sphinx>=8,<10" "furo>=2024.8.6"
   .venv/bin/maturin develop --manifest-path crates/python/Cargo.toml

On Windows with the repository's Conda environment:

.. code-block:: powershell

   conda activate COS
   maturin develop --uv --manifest-path crates/python/Cargo.toml

Platform Notes
--------------

Interactive native windows require a graphical desktop. Static rendering uses
an offscreen OpenGL path and can run on headless Linux. Google Colab is detected
automatically and uses an isolated software-rendering process; notebook code
does not need to set rendering environment variables.
