"""Exercise the workflow's actual publishing policy without uploading packages."""

import os
from pathlib import Path
import re
import textwrap
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]


class PublishPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / ".github/workflows/publish.yml").read_text(encoding="utf-8")
        match = re.search(r"          python - <<'PY'\n(.*?)          PY\n", cls.workflow, re.S)
        if match is None:
            raise AssertionError("Missing workflow publishing policy")
        cls.policy = compile(textwrap.dedent(match[1]), "<workflow publishing policy>", "exec")

    def test_publishing_policy(self):
        cases = [
            ("manual RC publish", "0.5.0-rc.2", "0.5.0rc2", "workflow_dispatch", "true", True, False),
            ("manual stable publish", "0.5.0", "0.5.0", "workflow_dispatch", "true", False, True),
            ("manual stable build", "0.5.0", "0.5.0", "workflow_dispatch", "false", False, False),
            ("stable tag push", "0.5.0", "0.5.0", "push", "", False, False),
            ("mismatched RCs", "0.5.0-rc.2", "0.5.0rc1", "workflow_dispatch", "true", False, True),
            ("stable Python", "0.5.0-rc.2", "0.5.0", "workflow_dispatch", "true", False, True),
            ("other prereleases", "0.5.0-beta.2", "0.5.0b2", "workflow_dispatch", "true", False, True),
            ("stable pull request", "0.5.0", "0.5.0", "pull_request", "", False, False),
        ]
        for name, rust, python, event, requested, allowed, rejected in cases:
            with self.subTest(name=name):
                files = mock.mock_open()
                versions = [
                    {"workspace": {"package": {"version": rust}}},
                    {"project": {"version": python}},
                ]
                env = {
                    "GITHUB_OUTPUT": "test-output",
                    "GITHUB_EVENT_NAME": event,
                    "PUBLISH_REQUESTED": requested,
                }
                with mock.patch("builtins.open", files), mock.patch("tomllib.load", side_effect=versions), mock.patch.dict(os.environ, env, clear=True):
                    if rejected:
                        with self.assertRaisesRegex(SystemExit, "Stable releases must be published by pushing a tag"):
                            exec(self.policy, {})
                    else:
                        exec(self.policy, {})
                files().write.assert_called_once_with(f"manual_publish_allowed={str(allowed).lower()}\n")

    def test_all_publish_steps_use_the_same_guard(self):
        condition = "if: ${{ (github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')) || (github.event_name == 'workflow_dispatch' && inputs.publish && needs.wasm.outputs.manual_publish_allowed == 'true') }}"
        self.assertEqual(self.workflow.count(condition), 3)
        self.assertIn("needs: [wasm, linux, windows, macos, sdist]", self.workflow)
        self.assertIn("manual_publish_allowed: ${{ steps.release_policy.outputs.manual_publish_allowed }}", self.workflow)


if __name__ == "__main__":
    unittest.main()
