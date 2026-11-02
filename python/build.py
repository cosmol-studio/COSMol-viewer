"""Build the extension and generate stubs with official PyO3 introspection."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

from framework.build import build_extension, extract_stubs, stage_extension

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--extractor", help="Path to the matching pyo3-introspection executable")
    parser.add_argument("--install-extractor", action="store_true", help="Install the official extractor matching the workspace PyO3 pin")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--check", action="store_true", help="Check runtime, registry, and consumer types")
    args = parser.parse_args()
    local = ROOT / "target/pyo3-tools/bin" / ("pyo3-introspection.exe" if os.name == "nt" else "pyo3-introspection")
    if args.install_extractor:
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        version = workspace["workspace"]["dependencies"]["pyo3"]["version"]
        if not version.startswith("="):
            raise SystemExit("Pin an exact workspace PyO3 version so the official extractor can match it")
        subprocess.run([
            "cargo", "install", "pyo3-introspection", "--version", version[1:],
            "--locked", "--root", str(ROOT / "target/pyo3-tools"),
        ], cwd=ROOT, check=True)
    extractor = args.extractor or os.environ.get("PYO3_INTROSPECTION") or (
        str(local) if local.is_file() else shutil.which("pyo3-introspection")
    )
    if not extractor:
        raise SystemExit("Install the matching official extractor: python python/build.py --install-extractor")
    manifest = HERE / "Cargo.toml"
    module = tomllib.loads(manifest.read_text(encoding="utf-8"))["lib"]["name"]
    library = build_extension(manifest, ROOT, release=args.release)
    stub = HERE / f"{module}.pyi"
    extract_stubs(library, module, extractor, stub)
    output = ROOT / "target/python-bindings"
    package = output / "package"
    stage_extension(library, module, stub, package)
    print(f"Official PyO3 declarations: {stub}", flush=True)
    if args.check:
        env = {**os.environ, "PYTHONPATH": os.pathsep.join([str(package), str(HERE)])}
        subprocess.run([
            sys.executable, "-m", "unittest", "discover", "-s", str(HERE / "tests"),
            "-p", "test_*.py", "-v",
        ], cwd=ROOT, env=env, check=True)
        config = output / "typing.json"
        config.write_text(json.dumps({
            "include": [str(HERE / "tests/typing"), str(stub)],
            "extraPaths": [str(package)], "pythonVersion": "3.10",
            "typeCheckingMode": "strict", "reportMissingModuleSource": False,
        }, indent=2), encoding="utf-8")
        subprocess.run([
            sys.executable, "-m", "basedpyright", "--pythonpath", sys.executable,
            "--project", str(config),
        ], cwd=ROOT, env=env, check=True)


if __name__ == "__main__":
    main()
