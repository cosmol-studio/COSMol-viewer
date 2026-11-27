"""Apply Windows release patches without changing Cargo's cache."""

import json
from pathlib import Path
import shutil
import subprocess


PATCHES = (
    ("surfman", "0.14.0", "surfman-0.14.0-quiet-wgl.patch"),
    ("iceoryx2-pal-posix", "0.10.1", "iceoryx2-pal-posix-0.10.1-remove-uds.patch"),
)


def prepare(root):
    config = root / ".cargo/config.toml"
    if config.exists() or (root / ".cargo/config").exists():
        raise RuntimeError("Refusing to overwrite an existing workspace Cargo configuration")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked",
         "--filter-platform", "x86_64-pc-windows-msvc"],
        cwd=root, text=True, encoding="utf-8",
    ))
    replacements = ["[patch.crates-io]"]
    update = ["cargo", "update", "--offline"]
    for name, version, patch_name in PATCHES:
        package, = [item for item in metadata["packages"] if item["name"] == name]
        if package["version"] != version or not (package["source"] or "").startswith("registry+"):
            raise RuntimeError(f"Review the release patch when changing {name} versions or sources")
        source = Path(package["manifest_path"]).parent
        destination = root / "target/release-patches" / f"{name}-{version}"
        shutil.copytree(source, destination)
        # Normalize checkout CRLF before passing the patch to Git on Windows.
        patch = (root / "tools/patches" / patch_name).read_text(encoding="utf-8").encode("utf-8")
        subprocess.run(["git", "apply", "--check", "-"], input=patch, cwd=destination, check=True)
        subprocess.run(["git", "apply", "-"], input=patch, cwd=destination, check=True)
        replacements.append(f"{name} = {{ path = {json.dumps(destination.as_posix())} }}")
        update.extend(["-p", name])
        print(f"Windows release builds use patched {name}: {destination}")
    config.parent.mkdir(exist_ok=True)
    config.write_text("\n".join(replacements) + "\n", encoding="utf-8")
    # The sources change from registry to path; update only this CI checkout's
    # lockfile before build_native_viewer.py invokes Cargo with --locked.
    subprocess.run(update, cwd=root, check=True)


if __name__ == "__main__":
    prepare(Path(__file__).resolve().parents[1])
