"""Compile and extract PyO3 declarations without business API knowledge.

No signature rewriting, name maps, or per-function exclusions belong here.
The extension's declarative module and FromPyObject/IntoPyObject types own the
surface. New bindings are discovered by the compiler and official extractor.
"""
from __future__ import annotations

import ast
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib


def build_extension(manifest: Path, workspace: Path, *, release: bool = False) -> Path:
    metadata = tomllib.loads(manifest.read_text(encoding="utf-8"))
    name = metadata["lib"]["name"]
    command = [
        "cargo", "build", "--locked", "--manifest-path", str(manifest),
        "--message-format=json-render-diagnostics",
    ]
    if release:
        command.append("--release")
    result = subprocess.run(
        command, cwd=workspace, env={**os.environ, "PYO3_PYTHON": sys.executable},
        text=True, encoding="utf-8", stdout=subprocess.PIPE,
    )
    result.check_returncode()
    libraries = [
        Path(filename)
        for line in result.stdout.splitlines()
        if (item := json.loads(line)).get("reason") == "compiler-artifact"
        and item["target"]["name"] == name
        and "cdylib" in item["target"]["crate_types"]
        for filename in item["filenames"]
        if Path(filename).suffix in {".so", ".dll", ".dylib"}
    ]
    if len(libraries) != 1:
        raise RuntimeError(f"Expected one extension artifact for {name}, got {libraries}")
    return libraries[0]


def extract_stubs(library: Path, module: str, extractor: str, destination: Path) -> None:
    """Stage the official output verbatim, only after successful extraction."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="pyo3-inspect-") as temporary:
        output = Path(temporary)
        subprocess.run([extractor, str(library), module, str(output)], check=True)
        source = output / "__init__.pyi"
        content = source.read_text(encoding="utf-8")
        tree = ast.parse(content)
        # Fail closed if a dynamic module or custom converter lost its types.
        if any(isinstance(node, ast.Name) and node.id in {"Any", "Incomplete"} for node in ast.walk(tree)):
            raise ValueError("Incomplete PyO3 declarations: use a declarative module and typed converters")
        # An annotation string is permitted by PyO3, but must also be valid Python.
        for node in ast.walk(tree):
            if isinstance(node, ast.arg) and isinstance(node.annotation, ast.Constant):
                if isinstance(node.annotation.value, str):
                    ast.parse(node.annotation.value, mode="eval")
        shutil.copyfile(source, destination)


def stage_extension(library: Path, module: str, stub: Path, directory: Path) -> None:
    """Provide a clean import root for tests, independent of an installed wheel."""
    directory.mkdir(parents=True, exist_ok=True)
    suffix = ".pyd" if os.name == "nt" else ".so"
    shutil.copy2(library, directory / (module + suffix))
    shutil.copy2(stub, directory / (module + ".pyi"))
