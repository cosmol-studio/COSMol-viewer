"""Build official wasm-bindgen declarations and check the resulting package."""
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


def validate(pkg: Path, env: dict[str, str], browser: bool) -> None:
    node = command("node", "NODE_BIN")
    # A fixed, independent consumer checks the converter mechanism without
    # exporting test-only types from the viewer or enumerating business APIs.
    converters = pkg.parent / "converters"
    run(command("wasm-pack", "WASM_PACK_BIN"), "build", str(WASM / "tests/converters"),
        "--target", "web", "--out-dir", str(converters), "--out-name", "converters",
        "--dev", "--no-opt", "--locked", cwd=ROOT, env=env)
    (converters / "package.json").write_text('{"type":"module","private":true}\n', encoding="utf-8")
    run(node, "--test", str(WASM / "tests/converters/runtime.mjs"), cwd=ROOT,
        env=dict(env, COSMOL_VIEWER_WASM_CONVERTERS=str(converters)))
    for case, boundary in (("converter_invalid_input", "CheckedValue"), ("converter_invalid_output", "TypedOutput")):
        result = subprocess.run([
            "cargo", "rustc", "--lib", "--locked", "--manifest-path", str(WASM / "tests/converters/Cargo.toml"),
            "--target", "wasm32-unknown-unknown", "--message-format=json", "--", "--cfg", case,
        ], cwd=ROOT, env=env, capture_output=True, text=True, encoding="utf-8")
        diagnostics = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
        if result.returncode == 0 or not any(
            row.get("reason") == "compiler-message"
            and (row["message"].get("code") or {}).get("code") == "E0277"
            and boundary in (row["message"]["message"] + (row["message"].get("rendered") or ""))
            for row in diagnostics
        ):
            raise RuntimeError(f"{case} did not fail at {boundary}:\n{result.stderr}\n{result.stdout}")
    runtime_env = dict(env, COSMOL_VIEWER_WASM_PKG=str(pkg))
    run(node, "--test", str(ROOT / "wasm/tests/binding_surface.mjs"), cwd=ROOT, env=runtime_env)
    if browser:
        run(node, "--test", str(ROOT / "wasm/tests/browser_render.mjs"), cwd=ROOT, env=runtime_env)
    with tempfile.TemporaryDirectory(prefix="cosmol-viewer-types-") as temporary:
        config = Path(temporary) / "tsconfig.json"
        config.write_text(json.dumps({
            "compilerOptions": {
                "strict": True, "noEmit": True, "target": "ES2022",
                "module": "ESNext", "moduleResolution": "Bundler",
                "lib": ["ES2022", "DOM", "ESNext.Disposable"],
                "paths": {
                    "cosmol-viewer-generated": [str(pkg / "cosmol_viewer_wasm.d.ts")],
                    "cosmol-viewer-converters": [str(converters / "converters.d.ts")],
                },
            },
            "files": [str(WASM / "tests/binding_surface.ts"), str(WASM / "tests/converters/types.ts")],
        }), encoding="utf-8")
        tsc = shutil.which("tsc")
        if tsc:
            run(tsc, "--project", str(config), cwd=ROOT, env=env)
        else:
            run(command("npx", "NPX_BIN"), "--yes", "--package", "typescript@5.8.3", "tsc", "--project", str(config), cwd=ROOT, env=env)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Run runtime and strict TypeScript checks")
    parser.add_argument("--browser-check", action="store_true", help="Also render in Chromium (requires --check)")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--no-opt", action="store_true", help="Skip release wasm-opt")
    options = parser.parse_args()
    if options.browser_check and not options.check:
        parser.error("--browser-check requires --check")
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
        package_json.write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
        if options.check:
            validate(package, env, options.browser_check)
        # Stage only after the build and all requested checks succeed.
        shutil.copytree(package, WASM / "pkg", dirs_exist_ok=True)
    print(f"Official browser package staged in {WASM / 'pkg'}")


if __name__ == "__main__":
    main()
