import os
import sys

sys.path.insert(0, os.path.abspath("../.."))

project = "cosmol_viewer"
author = "Jingtong Wang"

extensions = [
    "sphinx.ext.autodoc",
    "sphinx_autodoc_typehints",
    "sphinx.ext.autosummary",
    "sphinx.ext.napoleon",
    "sphinx.ext.viewcode",
]

autosummary_generate = True
autodoc_typehints = "signature"
autoclass_content = "both"
always_document_param_types = True

templates_path = ["_templates"]
exclude_patterns = []

html_theme = "furo"
