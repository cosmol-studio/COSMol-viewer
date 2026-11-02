"""Validate matching Rust/Python/WASM versions before building release artifacts."""
import os
from pathlib import Path
import tomllib

try:
    from .bump_version import release_versions
except ImportError:
    from bump_version import release_versions

ROOT = Path(__file__).resolve().parents[1]

def check_versions(rust: str, python: str, wasm: str, event: str, ref: str, requested: bool) -> bool:
    expected_rust, expected_python = release_versions(rust)
    if rust != expected_rust:
        raise ValueError("Rust release version must use canonical Cargo notation")
    if python != expected_python or wasm != rust:
        raise ValueError("Rust, Python and WASM release versions must match")
    if event == "push" and ref != f"refs/tags/v{rust}":
        raise ValueError("Release tag must equal v followed by the Rust version")
    prerelease = "-" in rust
    if event == "workflow_dispatch" and requested and not prerelease:
        raise ValueError("Manual publication is dev/RC-only; publish stable releases by pushing a tag")
    return prerelease

def main() -> None:
    def manifest(path):
        return tomllib.loads((ROOT / path).read_text(encoding="utf-8"))
    workspace = manifest("Cargo.toml")["workspace"]["package"]["version"]
    wasm_version = manifest("wasm/Cargo.toml")["package"]["version"]
    if wasm_version == {"workspace": True}:
        wasm_version = workspace
    try:
        allowed = check_versions(
            workspace, manifest("python/pyproject.toml")["project"]["version"], wasm_version,
            os.environ.get("GITHUB_EVENT_NAME", "workflow_dispatch"),
            os.environ.get("GITHUB_REF", ""),
            os.environ.get("PUBLISH_REQUESTED") == "true",
        )
    except ValueError as error:
        raise SystemExit(f"::error::{error}") from error
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a", encoding="utf-8") as destination:
            destination.write(f"manual_publish_allowed={str(allowed).lower()}\n")
    print(f"Release versions match: {workspace}; manual prerelease publication allowed: {allowed}")

if __name__ == "__main__":
    main()
