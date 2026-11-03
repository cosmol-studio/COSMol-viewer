"""Build official WASM outputs and stage the independent npm entry point."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
WASM = ROOT / "wasm"


def command(name: str, variable: str) -> str:
    executable = os.environ.get(variable) or shutil.which(name)
    if not executable:
        raise SystemExit(f"Install {name}, or set {variable} to its executable path")
    return executable


def run(*arguments: str, cwd: Path, env: dict[str, str]) -> None:
    subprocess.run(arguments, cwd=cwd, env=env, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--no-opt", action="store_true", help="Skip release wasm-opt")
    options = parser.parse_args()
    env = os.environ.copy()
    env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target"))
    with tempfile.TemporaryDirectory(prefix="cosmol-viewer-wasm-") as temporary:
        package = Path(temporary) / "pkg"
        arguments = ["build", str(WASM), "--target", "web", "--out-dir", str(package),
                     "--out-name", "cosmol_viewer_wasm",
                     "--release" if options.release else "--dev"]
        if options.no_opt or not options.release:
            arguments.append("--no-opt")
        # wasm-pack invokes the official extractor matching the compiled wasm-bindgen.
        run(command("wasm-pack", "WASM_PACK_BIN"), *arguments, "--locked", cwd=ROOT, env=env)
        metadata = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]
        package_json = package / "package.json"
        info = json.loads(package_json.read_text(encoding="utf-8"))
        info.update(name="@cosmol-studio/cosmol-viewer", type="module",
                    description=metadata["description"], license=metadata["license"],
                    repository={"type": "git", "url": metadata["repository"]})
        binding_metadata = tomllib.loads((WASM / "Cargo.toml").read_text(encoding="utf-8"))
        info.update(binding_metadata["package"].get("metadata", {}).get("javascript", {}))
        shutil.copytree(WASM / "js", package, dirs_exist_ok=True)
        info["main"] = "index.js"
        info["files"] = list(dict.fromkeys([*info["files"], "index.js"]))
        package_json.write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
        # The Notebook uses the unmodified official JS/WASM, without npm imports.
        shutil.copytree(package, WASM / "pkg", dirs_exist_ok=True,
                        ignore=shutil.ignore_patterns("node_modules", "package-lock.json"))
        (WASM / "pkg/notebook.js").unlink(missing_ok=True)
    print(f"Official browser package staged in {WASM / 'pkg'}")


if __name__ == "__main__":
    main()
