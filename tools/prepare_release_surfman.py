"""Apply the Windows release-only surfman patch without changing Cargo's cache."""

import json
from pathlib import Path
import shutil
import subprocess


PATCH = Path(__file__).resolve().parent / "patches/surfman-0.14.0-quiet-wgl.patch"


def prepare(root):
    config = root / ".cargo/config.toml"
    if config.exists() or (root / ".cargo/config").exists():
        raise RuntimeError("Refusing to overwrite an existing workspace Cargo configuration")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked",
         "--filter-platform", "x86_64-pc-windows-msvc"],
        cwd=root, text=True, encoding="utf-8",
    ))
    package, = [item for item in metadata["packages"] if item["name"] == "surfman"]
    if package["version"] != "0.14.0" or not package["source"].startswith("registry+"):
        raise RuntimeError("Review the release patch when changing surfman versions or sources")
    source = Path(package["manifest_path"]).parent
    destination = root / "target/release-patches/surfman-0.14.0"
    shutil.copytree(source, destination)
    # Normalize checkout CRLF before passing the patch to Git on Windows.
    patch = PATCH.read_text(encoding="utf-8").encode("utf-8")
    subprocess.run(["git", "apply", "--check", "-"], input=patch, cwd=destination, check=True)
    subprocess.run(["git", "apply", "-"], input=patch, cwd=destination, check=True)
    config.parent.mkdir(exist_ok=True)
    config.write_text(
        '[patch.crates-io]\nsurfman = { path = '
        + json.dumps(destination.as_posix()) + ' }\n', encoding="utf-8",
    )
    # The source changes from registry to path; update only this CI checkout's
    # lockfile before build_native_viewer.py invokes Cargo with --locked.
    subprocess.run(["cargo", "update", "--offline", "-p", "surfman"], cwd=root, check=True)
    print(f"Windows release builds use patched surfman: {destination}")


if __name__ == "__main__":
    prepare(Path(__file__).resolve().parents[1])
