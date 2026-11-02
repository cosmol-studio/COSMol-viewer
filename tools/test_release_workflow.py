"""Validate the actual manual GitHub Release steps without remote mutations."""
import json
import os
from pathlib import Path
import re
import subprocess
import textwrap
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]

class ReleaseWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        match = re.search(r"          python - <<'PY'\n(.*?)          PY\n", cls.workflow, re.S)
        if match is None:
            raise AssertionError("Missing release validation and changelog extraction")
        cls.validation = compile(textwrap.dedent(match[1]), "<release workflow>", "exec")

    def run_validation(self, tag_version, rust, python, *, notes_version=None):
        notes_version = notes_version or tag_version
        content = {
            "Cargo.toml": f'[workspace.package]\nversion = "{rust}"\n',
            "python/pyproject.toml": f'[project]\nversion = "{python}"\n',
            "wasm/Cargo.toml": '[package]\nversion.workspace = true\n',
            "CHANGELOG.md": (
                "<!-- release-header:start -->\nProject links\n<!-- release-header:end -->\n\n"
                f"## [{notes_version}] - 2026-11-02\n\n### Changed\n\n- Release details.\n\n"
                "<!-- release-footer:start -->\nDocumentation links\n<!-- release-footer:end -->\n"
            ),
        }
        def read(path, **kwargs):
            return content[str(path).replace("\\", "/")]
        with mock.patch.dict(os.environ, {"RELEASE_TAG": f"v{tag_version}"}), \
             mock.patch.object(Path, "read_text", read), \
             mock.patch.object(Path, "write_text") as save:
            exec(self.validation, {})
            save.assert_called_once()
            return save.call_args.args[0]

    def test_rc_rust_and_python_versions_are_equivalent(self):
        notes = self.run_validation("0.6.0-rc.1", "0.6.0-rc.1", "0.6.0rc1")
        self.assertIn("COSMol Viewer v0.6.0-rc.1", notes)
        self.assertIn("- Release details.", notes)
        self.assertEqual(notes.count("Documentation links"), 1)

    def test_stable_versions_still_work(self):
        self.assertIn("COSMol Viewer v0.6.0", self.run_validation("0.6.0", "0.6.0", "0.6.0"))

    def test_mismatched_python_release_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "must match"):
            self.run_validation("0.6.0-rc.1", "0.6.0-rc.1", "0.6.0rc2")

    def test_mismatched_tag_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Release tag"):
            self.run_validation("0.6.0-rc.1", "0.6.0-rc.2", "0.6.0rc2")

    def test_missing_changelog_is_rejected(self):
        with self.assertRaisesRegex(SystemExit, "no dated section"):
            self.run_validation("0.6.0-rc.1", "0.6.0-rc.1", "0.6.0rc1", notes_version="0.5.0")

    def test_release_is_manual_and_prereleases_never_become_latest(self):
        self.assertIn("workflow_dispatch:", self.workflow)
        self.assertNotIn("workflow_run:", self.workflow)
        self.assertIn("--prerelease --latest=false", self.workflow)
        self.assertIn('gh release create "$RELEASE_TAG" --verify-tag', self.workflow)

    def test_only_one_unreleased_tag_on_selected_commit_is_used(self):
        match = re.search(r"          script: \|\n(.*?)(?=\n      - name: Checkout)", self.workflow, re.S)
        self.assertIsNotNone(match)
        code = textwrap.dedent(match[1])
        harness = """
const {code} = JSON.parse(require('node:fs').readFileSync(0, 'utf8'));
const run = new (Object.getPrototypeOf(async function(){}).constructor)('github','context','core',code);
const tag = (name,sha='head') => ({name,commit:{sha}});
const cases = [
  {tags:[],releases:[],expected:null},
  {tags:[tag('v0.6.0-rc.1','other')],releases:[],expected:null},
  {tags:[tag('v0.6.0-rc.1')],releases:[{tag_name:'v0.6.0-rc.1',draft:true}],expected:null},
  {tags:[tag('v0.6.0-rc.1')],releases:[],expected:'v0.6.0-rc.1'},
  {tags:[tag('v0.6.0-rc.1'),tag('v0.6.0-rc.2')],releases:[],failed:true},
  {tags:[tag('v0.6.0-rc.1'),tag('v0.6.0-rc.2')],releases:[],ref:'refs/tags/v0.6.0-rc.2',expected:'v0.6.0-rc.2'},
];
(async()=>{
  for (const c of cases) {
    let actual=null,failed=false;
    const github={rest:{repos:{listTags:'tags',listReleases:'releases'}},paginate:async key=>c[key]};
    const context={sha:'head',ref:c.ref||'refs/heads/main',repo:{owner:'test',repo:'viewer'}};
    const core={notice(){},info(){},setFailed(){failed=true;},setOutput(key,value){if(key==='tag')actual=value;}};
    await run(github,context,core);
    require('node:assert/strict').equal(failed,!!c.failed);
    require('node:assert/strict').equal(actual,c.expected||null);
  }
})().catch(error=>{console.error(error);process.exitCode=1;});
"""
        result = subprocess.run(["node", "-e", harness], input=json.dumps({"code": code}),
                                text=True, encoding="utf-8", capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

if __name__ == "__main__":
    unittest.main()
