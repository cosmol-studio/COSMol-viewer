"""Exercise the shared publish policy without uploading packages."""
import importlib.util
import os
import re
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from publish_policy import check_versions
import publish_policy

ROOT = Path(__file__).resolve().parents[1]

class PublishPolicyTests(unittest.TestCase):
    def test_publishing_policy(self):
        cases = [
            ("manual RC publish", "0.6.0-rc.1", "0.6.0rc1", "workflow_dispatch", True, True),
            ("manual RC build", "0.6.0-rc.1", "0.6.0rc1", "workflow_dispatch", False, True),
            ("manual dev publish", "0.6.0-dev.1", "0.6.0.dev1", "workflow_dispatch", True, True),
            ("manual stable build", "0.6.0", "0.6.0", "workflow_dispatch", False, False),
            ("stable tag push", "0.6.0", "0.6.0", "push", False, False),
            ("RC tag push", "0.6.0-rc.1", "0.6.0rc1", "push", False, True),
            ("stable pull request", "0.6.0", "0.6.0", "pull_request", False, False),
        ]
        for name, rust, python, event, requested, allowed in cases:
            with self.subTest(name=name):
                ref = f"refs/tags/v{rust}" if event == "push" else "refs/heads/main"
                self.assertEqual(check_versions(rust, python, rust, event, ref, requested), allowed)

    def test_stable_manual_publication_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "stable releases by pushing a tag"):
            check_versions("0.6.0", "0.6.0", "0.6.0", "workflow_dispatch", "", True)

    def test_versions_and_tag_must_match(self):
        for python, wasm in (("0.6.0rc2", "0.6.0-rc.1"), ("0.6.0", "0.6.0-rc.1"), ("0.6.0rc1", "0.6.0-rc.2")):
            with self.subTest(python=python, wasm=wasm), self.assertRaisesRegex(ValueError, "must match"):
                check_versions("0.6.0-rc.1", python, wasm, "workflow_dispatch", "", True)
        with self.assertRaisesRegex(ValueError, "Release tag"):
            check_versions("0.6.0-rc.1", "0.6.0rc1", "0.6.0-rc.1", "push", "refs/tags/v0.6.0-rc.2", False)
        for version in ("0.6.0-beta.2", "0.6.0rc1"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                check_versions(version, "0.6.0rc1", version, "workflow_dispatch", "", True)

    def test_cli_resolves_inherited_wasm_version_and_writes_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "python").mkdir()
            (root / "wasm").mkdir()
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.6.0-rc.1"\n', encoding="utf-8")
            (root / "python/pyproject.toml").write_text('[project]\nversion = "0.6.0rc1"\n', encoding="utf-8")
            (root / "wasm/Cargo.toml").write_text('[package]\nversion.workspace = true\n', encoding="utf-8")
            output = root / "outputs"
            with mock.patch.object(publish_policy, "ROOT", root), mock.patch.dict(os.environ, {
                "GITHUB_EVENT_NAME": "workflow_dispatch", "PUBLISH_REQUESTED": "true", "GITHUB_OUTPUT": str(output)
            }, clear=True):
                publish_policy.main()
            self.assertEqual(output.read_text(encoding="utf-8"), "manual_publish_allowed=true\nconda_label=rc\n")

    def test_publish_job_waits_for_all_artifacts_and_has_one_guard(self):
        workflow = (ROOT / ".github/workflows/publish.yml").read_text(encoding="utf-8")
        condition = "if: ${{ (github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')) || (github.event_name == 'workflow_dispatch' && inputs.publish && needs.generate-stubs.outputs.manual_publish_allowed == 'true') }}"
        self.assertEqual(workflow.count(condition), 1)
        publish = workflow.split("\n    publish:", 1)[1]
        needs = publish.split("        needs:\n", 1)[1].split("\n\n", 1)[0]
        self.assertEqual(set(line.strip()[2:] for line in needs.splitlines()),
                         {"build-wasm", "generate-stubs", "build-wheels", "build-sdist"})
        self.assertIn("python tools/publish_policy.py", workflow)
        self.assertIn("pattern: wheels-*", workflow)
        self.assertIn("merge-multiple: true", workflow)
        self.assertIn("name: sdist", workflow)
        self.assertIn("npm pack --dry-run --json", workflow)
        self.assertIn("fail-fast: false", workflow)

    def test_five_platform_jobs_reuse_stubs_and_renderer(self):
        workflow = (ROOT / ".github/workflows/publish.yml").read_text(encoding="utf-8")
        wheels = workflow.split("\n    build-wheels:", 1)[1].split("\n    build-sdist:", 1)[0]
        matrix = wheels.split("            matrix:\n", 1)[1].split("\n        steps:", 1)[0]
        self.assertEqual(re.findall(r"- name: (\S+)", matrix),
                         ["linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-aarch64", "windows-x86_64"])
        self.assertNotIn("matrix.python", workflow)
        self.assertNotIn("--generate-stubs", workflow)
        self.assertNotIn("sccache: false", wheels)
        self.assertEqual(wheels.count("build_native_viewer.py"), 2)  # Host or manylinux, once per platform.
        self.assertEqual(wheels.count("before-script-linux:"), 1)
        self.assertIn("--interpreter python3.11 python3.12 python3.13", wheels)
        stubs = workflow.split("\n    generate-stubs:", 1)[1].split("\n    build-wasm:", 1)[0]
        self.assertIn("needs: [build-wasm]", stubs)
        self.assertIn("name: wasm-pkg", stubs)
        self.assertNotIn("touch ", stubs)
        self.assertNotIn("skip-existing", workflow)
        self.assertNotIn("NPM_TOKEN", workflow)
        self.assertIn("id-token: write", workflow)
        self.assertIn("python tools/package_wheels.py dist conda-dist", workflow)

if __name__ == "__main__":
    unittest.main()
