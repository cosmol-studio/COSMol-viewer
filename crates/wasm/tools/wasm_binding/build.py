"""Generate browser bindings with Alef, build/stage the npm package, and check it."""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
from compat import preserve_source_semantics

ROOT = Path(__file__).resolve().parents[4]
CONFIG = Path(__file__).with_name("alef.toml")
PKG = ROOT / "crates" / "wasm" / "pkg"


def command(name: str, variable: str) -> str:
    executable = os.environ.get(variable) or shutil.which(name)
    if not executable:
        raise SystemExit(f"Install {name}, or set {variable} to its executable path")
    return executable


def run(*arguments: str, cwd: Path, env: dict[str, str]) -> None:
    subprocess.run(arguments, cwd=cwd, env=env, check=True)


def validate(pkg: Path, env: dict[str, str], browser: bool) -> None:
    node = command("node", "NODE_BIN")
    runtime_env = dict(env, COSMOL_VIEWER_WASM_PKG=str(pkg))
    run(node, "--test", str(ROOT / "crates/wasm/tests/binding_surface.mjs"), cwd=ROOT, env=runtime_env)
    if browser:
        run(node, "--test", str(ROOT / "crates/wasm/tests/browser_render.mjs"), cwd=ROOT, env=runtime_env)
    with tempfile.TemporaryDirectory(prefix="cosmol-viewer-types-") as temporary:
        config = Path(temporary) / "tsconfig.json"
        config.write_text(json.dumps({
            "compilerOptions": {
                "strict": True, "noEmit": True, "target": "ES2022",
                "module": "ESNext", "moduleResolution": "Bundler",
                "lib": ["ES2022", "DOM", "ESNext.Disposable"],
                "paths": {"cosmol-viewer-generated": [str(pkg / "cosmol_viewer_wasm.d.ts")]},
            },
            "files": [str(ROOT / "crates/wasm/tests/binding_surface.ts")],
        }), encoding="utf-8")
        tsc = shutil.which("tsc")
        if tsc:
            run(tsc, "--project", str(config), cwd=ROOT, env=env)
        else:
            run(command("npx", "NPX_BIN"), "--yes", "--package", "typescript@5.8.3", "tsc", "--project", str(config), cwd=ROOT, env=env)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Run actual JS and TypeScript checks after building")
    parser.add_argument("--browser-check", action="store_true", help="Also verify WebGL rendering with Playwright (requires --check)")
    parser.add_argument("--generate-only", action="store_true", help="Validate Alef output without building/staging a package")
    parser.add_argument("--no-opt", action="store_true", help="Skip wasm-opt for a faster development build")
    options = parser.parse_args()
    if options.browser_check and not options.check:
        parser.error("--browser-check requires --check")
    alef = command("alef", "ALEF_BIN")
    version = subprocess.check_output([alef, "--version"], text=True, encoding="utf-8").splitlines()[0]
    config_text = CONFIG.read_text(encoding="utf-8")
    metadata = tomllib.loads(config_text)
    expected_version = metadata["workspace"]["alef_version"]
    if version != f"alef {expected_version}":
        raise SystemExit(f"Expected Alef {expected_version}, got {version}")
    env = os.environ.copy()
    # The generator's scaffold toolchain is not the consumer's chosen toolchain.
    env.setdefault("RUSTUP_TOOLCHAIN", "stable")
    env["CARGO_TARGET_DIR"] = str(ROOT / "target")
    with tempfile.TemporaryDirectory(prefix="cosmol-viewer-alef-") as temporary:
        workspace = Path(temporary)
        generated = workspace / "generated"
        if (ROOT / "LICENSE").is_file():
            shutil.copy2(ROOT / "LICENSE", workspace / "LICENSE")
        # Absolute inputs keep generation isolated without Windows symlink privileges.
        isolated = config_text
        for source in metadata["crates"][0]["sources"]:
            isolated = isolated.replace(json.dumps(source), json.dumps((ROOT / source).as_posix()))
        isolated = isolated.replace('"Cargo.toml"', json.dumps((ROOT / "Cargo.toml").as_posix())).replace(
            '"target/alef/wasm"', '"generated"'
        )
        config = workspace / "alef.toml"
        config.write_text(isolated, encoding="utf-8")
        run(alef, "--config", str(config), "generate", "--crate", "cosmol_viewer_wasm", "--lang", "wasm", cwd=workspace, env=env)
        # Alef derives a relative crate path incorrectly from absolute
        # source paths on Windows. Bind the generated adapter to the real crate.
        manifest = generated / "Cargo.toml"
        manifest_text, count = re.subn(
            r'^cosmol_viewer_wasm = .*$',
            'cosmol_viewer_wasm = { path = ' + json.dumps((ROOT / "crates/wasm").as_posix())
            + ', default-features = false, features = ["wasm"] }',
            manifest.read_text(encoding="utf-8"), flags=re.MULTILINE,
        )
        if count != 1:
            raise SystemExit("Alef generated an unexpected source dependency manifest")
        wasm_metadata = tomllib.loads((ROOT / "crates/wasm/Cargo.toml").read_text(encoding="utf-8"))
        release_flags = wasm_metadata["package"]["metadata"]["wasm-pack"]["profile"]["release"]["wasm-opt"]
        manifest_text, count = re.subn(
            r'^wasm-opt = false$', 'wasm-opt = ' + json.dumps(release_flags),
            manifest_text, flags=re.MULTILINE,
        )
        if count != 1:
            raise SystemExit("Alef generated an unexpected wasm-pack release profile")
        manifest.write_text(manifest_text, encoding="utf-8")
        rust_source = generated / "src/lib.rs"
        api = json.loads((workspace / ".alef/cosmol_viewer_wasm/ir.json").read_text(encoding="utf-8"))
        source = preserve_source_semantics(rust_source.read_text(encoding="utf-8"), api)
        rust_source.write_text(source, encoding="utf-8")
        debug_output = ROOT / "target/alef"
        debug_output.mkdir(parents=True, exist_ok=True)
        shutil.copy2(generated / "src/lib.rs", debug_output / "last-binding.rs")
        shutil.copy2(workspace / ".alef/cosmol_viewer_wasm/ir.json", debug_output / "api-ir.json")
        if "self.inner.lock()" in source:
            raise SystemExit("Browser handles must be shared/cloneable; an adapter mutex across await would block dispatch")
        for method in ["binding_contract_json", "new", "render", "play", "update", "dispatch", "render_notebook", "play_notebook", "take_screenshot", "add_frame", "set_static_scene"]:
            if f"fn {method}(" not in source:
                raise SystemExit(f"Alef omitted required browser API: {method}")
        if options.generate_only:
            print("Alef browser API generation passed")
            return
        # Seed resolution with consumer pins rather than updating unrelated dependencies.
        shutil.copy2(ROOT / "Cargo.lock", generated / "Cargo.lock")
        build_args = ["build", str(generated), "--target", "web", "--out-dir", "pkg", "--out-name", "cosmol_viewer_wasm", "--release"]
        if options.no_opt:
            build_args.append("--no-opt")
        run(command("wasm-pack", "WASM_PACK_BIN"), *build_args, cwd=workspace, env=env)
        package = generated / "pkg"
        package_json = package / "package.json"
        info = json.loads(package_json.read_text(encoding="utf-8"))
        info["name"] = metadata["crates"][0]["wasm"]["package_name"]
        root_package = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]
        info["description"] = root_package["description"]
        info["license"] = root_package["license"]
        info["repository"] = {"type": "git", "url": "https://github.com/cosmol-studio/COSMol-viewer"}
        package_json.write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
        if options.check:
            validate(package, env, options.browser_check)
        # Publish only a complete package after generation/build/check succeeded.
        PKG.mkdir(parents=True, exist_ok=True)
        for artifact in package.iterdir():
            if artifact.is_file():
                shutil.copy2(artifact, PKG / artifact.name)
    print(f"Alef-generated browser package staged in {PKG}")


if __name__ == "__main__":
    main()
