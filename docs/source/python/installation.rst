.. meta::
   :description: Install COSMol Viewer from PyPI, verify the installation, and choose a desktop, notebook, or headless rendering environment.

Installation
============

Install the Package
-------------------

Install the released Python package from PyPI:

.. code-block:: bash

   python -m pip install cosmol-viewer

The wheel contains the native Python extension and the WebAssembly assets used
by interactive Jupyter and Google Colab viewers. A separate JavaScript install
is not required for Python use.

Verify the Installation
-----------------------

Create a scene to check that the package can be imported:

.. code-block:: python

   from cosmol_viewer import Scene

   scene = Scene()
   print("COSMol Viewer is ready")

Continue with :doc:`quickstart` to load a molecule and create an image.

Platform Notes
--------------

Interactive native windows require a graphical desktop. Static rendering uses
an offscreen OpenGL path and can run on headless Linux. Google Colab is detected
automatically and uses an isolated software-rendering process; notebook code
does not need to set rendering environment variables.
