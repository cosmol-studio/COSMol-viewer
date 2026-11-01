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
   uv pip install --python .venv/bin/python "maturin==1.15.0" "sphinx>=8,<10" "furo>=2024.8.6"
   .venv/bin/python python/build_native_viewer.py
   .venv/bin/maturin develop --generate-stubs --manifest-path python/Cargo.toml

On Windows with the repository's Conda environment:

.. code-block:: powershell

   conda activate COS
   python -m pip install "maturin==1.15.0"
   python python/build_native_viewer.py
   maturin develop --uv --generate-stubs --manifest-path python/Cargo.toml

Type declarations are generated from the compiled PyO3 bindings. For binding
development, run ``python python/build.py --install-extractor --check``
from the repository root; see ``python/DEV.md`` for development tools.

Platform Notes
--------------

Interactive native windows require a graphical desktop. Static rendering uses
an offscreen OpenGL path and can run on headless Linux. Google Colab is detected
automatically and uses an isolated software-rendering process; notebook code
does not need to set rendering environment variables.
