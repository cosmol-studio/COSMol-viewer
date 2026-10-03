"""Build and stage the native renderer before a maturin CLI build."""

import argparse
import json
from pathlib import Path
import shutil
import subprocess


def build_native_viewer(profile="dev", target=None, features=None):
    root = Path(__file__).resolve().parent
    # maturin's sdist places pyproject.toml and these hooks at the workspace root.
    manifest = root.parent / "core" / "Cargo.toml"
    if not manifest.is_file():
        manifest = root / "crates" / "core" / "Cargo.toml"
    command = [
        "cargo", "build", "--manifest-path", str(manifest),
        "--bin", "cosmol-viewer-native", "--profile", profile,
        "--message-format=json-render-diagnostics", "--locked",
    ]
    if target:
        command += ["--target", target]
    if features:
        command += ["--features", features]
    process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    executable = None
    for line in process.stdout:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            print(line, end="")
            continue
        if message.get("reason") == "compiler-message":
            print(message["message"].get("rendered", ""), end="")
        elif message.get("reason") == "compiler-artifact":
            if message["target"]["name"] == "cosmol-viewer-native" and message.get("executable"):
                executable = Path(message["executable"])
    if process.wait() != 0:
        raise subprocess.CalledProcessError(process.returncode, command)
    if executable is None:
        raise RuntimeError("Cargo did not produce the native viewer executable")
    destination = root / "cosmol_viewer_native" / executable.name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(executable, destination)
    print(f"Staged native viewer: {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", default="dev")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--target")
    parser.add_argument("--features")
    options = parser.parse_args()
    build_native_viewer("release" if options.release else options.profile, options.target, options.features)
