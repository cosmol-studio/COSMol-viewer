"""Release bump regressions; actual repository files and dependencies stay untouched."""

import contextlib
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("bump_version", ROOT / "tools/bump_version.py")
bump = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bump)


class ReleaseBumpTests(unittest.TestCase):
    def setUp(self):
        self.original = bump.read_release_files(ROOT)

    def test_python_and_rust_candidate_forms_are_equivalent(self):
        python = bump.prepare_updates(self.original, "0.5.0rc1")
        rust = bump.prepare_updates(self.original, "0.5.0-rc.1")
        self.assertEqual(python, rust)
        manifest = tomllib.loads(python["Cargo.toml"])
        self.assertEqual(manifest["workspace"]["package"]["version"], "0.5.0-rc.1")
        for name in bump.INTERNAL_DEPENDENCIES:
            self.assertEqual(manifest["workspace"]["dependencies"][name]["version"], "0.5.0-rc.1")
        self.assertEqual(tomllib.loads(python["crates/python/pyproject.toml"])["project"]["version"], "0.5.0rc1")

    def test_stable_release_and_idempotence(self):
        updated = bump.prepare_updates(self.original, "0.5.0")
        self.assertEqual(tomllib.loads(updated["Cargo.toml"])["workspace"]["package"]["version"], "0.5.0")
        self.assertEqual(tomllib.loads(updated["crates/python/pyproject.toml"])["project"]["version"], "0.5.0")
        self.assertEqual(bump.prepare_updates(updated, "0.5.0"), updated)

    def test_external_dependencies_and_other_fields_are_unchanged(self):
        updated = bump.prepare_updates(self.original, "0.5.1rc10")
        expected = tomllib.loads(self.original["Cargo.toml"])
        actual = tomllib.loads(updated["Cargo.toml"])
        expected["workspace"]["package"]["version"] = "0.5.1-rc.10"
        for name in bump.INTERNAL_DEPENDENCIES:
            expected["workspace"]["dependencies"][name]["version"] = "0.5.1-rc.10"
        self.assertEqual(actual, expected)
        expected = tomllib.loads(self.original["crates/python/pyproject.toml"])
        expected["project"]["version"] = "0.5.1rc10"
        self.assertEqual(tomllib.loads(updated["crates/python/pyproject.toml"]), expected)
        self.assertEqual(set(updated), set(bump.FILES))

    def test_release_crates_inherit_the_workspace_version(self):
        for member in ("cosmol_viewer", "crates/core", "crates/python", "crates/wasm"):
            with self.subTest(member=member):
                manifest = tomllib.loads((ROOT / member / "Cargo.toml").read_text(encoding="utf-8"))
                self.assertEqual(manifest["package"]["version"], {"workspace": True})

    def test_every_versioned_internal_path_dependency_is_covered(self):
        workspace = tomllib.loads(self.original["Cargo.toml"])["workspace"]
        found = {
            name for name, dependency in workspace["dependencies"].items()
            if isinstance(dependency, dict) and "path" in dependency and "version" in dependency
        }
        self.assertEqual(found, set(bump.INTERNAL_DEPENDENCIES))
        for member in workspace["members"]:
            manifest = tomllib.loads((ROOT / member / "Cargo.toml").read_text(encoding="utf-8"))
            for section in ("dependencies", "dev-dependencies", "build-dependencies"):
                for name, dependency in manifest.get(section, {}).items():
                    self.assertFalse(
                        isinstance(dependency, dict) and "path" in dependency and "version" in dependency,
                        (member, section, name),
                    )

    def test_comments_utf8_and_newlines_are_preserved(self):
        for newline in ("\n", "\r\n"):
            with self.subTest(newline=newline):
                text = '[package]\n  version = "0.1.0" # 版本\nname = "example"\n[dependencies]\nx = "0.1.0"\n'.replace("\n", newline)
                self.assertEqual(
                    bump.update_field(text, "package", "version", "0.5.0-rc.1"),
                    text.replace('version = "0.1.0"', 'version = "0.5.0-rc.1"'),
                )

    def test_missing_field_and_unexpected_dependency_fail_before_writes(self):
        for broken in ("missing", "path"):
            with self.subTest(broken=broken):
                original = self.original.copy()
                if broken == "missing":
                    version = tomllib.loads(original["crates/python/pyproject.toml"])["project"]["version"]
                    original["crates/python/pyproject.toml"] = original["crates/python/pyproject.toml"].replace(f'version = "{version}"', "", 1)
                else:
                    original["Cargo.toml"] = original["Cargo.toml"].replace('path = "crates/core"', 'path = "unexpected"')
                with self.assertRaises(ValueError):
                    bump.prepare_updates(original, "0.5.0rc1")
        self.assertEqual(self.original, bump.read_release_files(ROOT))

    def test_invalid_versions_are_rejected(self):
        for version in ("0.5", "0.5.0rc", "0.5.0rc01", "0.5.0-rc.01", "00.5.0", "0.5.0-beta.1", "0.5.0+build", "０.5.0", "0.5.0rc1\n"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                bump.release_versions(version)

    def test_cli_dry_run_and_invalid_input_never_write(self):
        path = ROOT / "tools/bump_version.py"
        result = subprocess.run([sys.executable, str(path), "0.99.0rc1", "--dry-run"], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--- Cargo.toml", result.stdout)
        self.assertIn("--- crates/python/pyproject.toml", result.stdout)
        self.assertIn("Would then run cargo update --workspace.", result.stdout)
        result = subprocess.run([sys.executable, str(path), "0.5"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self.original, bump.read_release_files(ROOT))

    def test_apply_and_lock_failure_in_a_temporary_repository(self):
        for returncode in (0, 1):
            with self.subTest(returncode=returncode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for file, text in self.original.items():
                    (root / file).parent.mkdir(parents=True, exist_ok=True)
                    with (root / file).open("w", encoding="utf-8", newline="") as stream:
                        stream.write(text)
                with mock.patch.object(bump, "ROOT", root), mock.patch.object(bump.subprocess, "run") as run:
                    run.return_value.returncode = returncode
                    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                        if returncode:
                            with self.assertRaises(SystemExit) as error:
                                bump.main(["0.5.0rc1"])
                            self.assertEqual(error.exception.code, 1)
                        else:
                            bump.main(["0.5.0rc1"])
                    run.assert_called_once_with(["cargo", "update", "--workspace"], cwd=root)
                self.assertEqual(bump.read_release_files(root), bump.prepare_updates(self.original, "0.5.0rc1"))


if __name__ == "__main__":
    unittest.main()
