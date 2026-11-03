"""Generated native API documentation must retain the checked stub's types."""

import io
import json
import pickle
from pathlib import Path
import sys
import subprocess
import tempfile
from types import SimpleNamespace
import unittest

from sphinx.application import Sphinx


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
sys.path.insert(0, str(DOCS))
from api_types import public_description, read_stub, return_signature, type_text


class DocumentationWorkflowTests(unittest.TestCase):
    def test_language_indexes_do_not_mix_entries(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "conf.py").write_text(
                f"import sys\nsys.path.insert(0, {str(DOCS)!r})\n"
                "extensions = ['api_index']\nhtml_use_index = False\n"
                "exclude_patterns = ['html', 'doctrees']\n")
            (root / "index.rst").write_text(
                "API\n===\n\n.. toctree::\n\n   python/api\n   javascript/api\n")
            for language, domain in (("python", "py"), ("javascript", "js")):
                (root / language).mkdir()
                (root / language / "api.rst").write_text(
                    f"API\n===\n\n.. {domain}:class:: Sample\n\n"
                    f".. {domain}:function:: select(value)\n")
            result = subprocess.run([
                sys.executable, str(DOCS / "sphinx_build.py"), "-W", "-b", "html",
                "-d", str(root / "doctrees"), str(root), str(root / "html"),
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            from bs4 import BeautifulSoup
            for language, other in (("python", "javascript"), ("javascript", "python")):
                html = BeautifulSoup((root / "html" / language / "genindex.html").read_text(), "html.parser")
                links = [a['href'] for a in html.select('.genindextable a[href]')]
                self.assertTrue(links)
                self.assertTrue(all(link.startswith('api.html#') for link in links), links)
                self.assertFalse(any(other in link for link in links), links)
                self.assertIn('Sample', html.get_text())
                self.assertIn('select', html.get_text())

    def test_javascript_optional_parameters_survive_rendering(self):
        # One protocol fixture, not a business API inventory. Required union
        # types must stay required; defaults and rest parameters stay intact.
        with tempfile.TemporaryDirectory(dir=DOCS, prefix=".api-test-") as folder:
            root = Path(folder)
            (root / "sample.ts").write_text('''
export class Example {
    constructor(options?: { enabled: boolean } | null) {}
    choose(required: string | undefined, optional?: number): void {}
    defaults(count = 3, ...names: string[]): void {}
}
export function query(required: string | null, optional?: boolean): number { return 0; }
export const alias: typeof query = query;
export class FactoryOnly {
    private constructor() {}
    static create(): FactoryOnly { return new FactoryOnly(); }
}
''')
            hook = (DOCS / "sphinx-js.config.ts").read_text().replace(
                'new URL("../wasm/pkg/enums.rst", import.meta.url)',
                f'new URL({json.dumps((root / "enums.rst").as_uri())})',
            )
            (root / "sphinx-js.config.ts").write_text(hook)
            (root / "tsconfig.json").write_text(json.dumps({
                "compilerOptions": {"target": "ES2022", "strict": True},
                "include": ["sample.ts"],
            }))
            (root / "conf.py").write_text(
                f"import sys\nsys.path.insert(0, {str(DOCS)!r})\n"
                "extensions = ['sphinx_js', 'api_layout']\n"
                "js_language = 'typescript'\njs_source_path = 'sample.ts'\n"
                "root_for_relative_js_paths = '.'\n"
                "jsdoc_tsconfig_path = 'tsconfig.json'\n"
                f"ts_sphinx_js_config = {str(root / 'sphinx-js.config.ts')!r}\n"
                "exclude_patterns = ['enums.rst', 'html', 'doctrees']\n"
            )
            (root / "index.rst").write_text(
                "API\n===\n\n.. js:autoclass:: Example\n   :members:\n\n"
                ".. js:autofunction:: query\n\n.. js:autofunction:: alias\n"
                "\n.. js:autoclass:: FactoryOnly\n"
            )
            result = subprocess.run([
                sys.executable, str(DOCS / "sphinx_build.py"), "-W", "--keep-going", "-E",
                "-b", "html", "-d", str(root / "doctrees"), str(root), str(root / "html"),
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            from sphinx import addnodes
            tree = pickle.loads((root / "doctrees/index.doctree").read_bytes())
            signatures = [n.astext() for n in tree.findall(addnodes.desc_signature)]
            self.assertTrue(any("options?:" in s and "null" in s for s in signatures), signatures)
            for name in ("choose", "query", "alias"):
                signature = next(s for s in signatures if f"{name}(" in s)
                self.assertIn("optional?:", signature)
                self.assertIn("required:", signature)
                self.assertNotIn("required?", signature)
            default = next(s for s in signatures if "defaults(" in s)
            self.assertIn("count=3", default)
            self.assertNotIn("count?", default)
            self.assertIn("...names", default)
            factory = next(s for s in signatures if s.endswith("FactoryOnly"))
            self.assertNotIn("(", factory)

    def test_projection_metadata_preserves_help(self):
        lines = ['{"role":"return_type","type":"list[int]"}', "", "Return selected values."]
        public_description(None, "method", "cosmol_viewer.Sample.values", None, None, lines)
        self.assertEqual(lines, ["Return selected values."])
        prose = ['{"example": 2}', "This is a documented data example."]
        public_description(None, "method", "cosmol_viewer.Sample.example", None, None, prose)
        self.assertEqual(prose, ['{"example": 2}', "This is a documented data example."])

    def test_generates_stub_before_documentation_consumers(self):
        workflow = (ROOT / ".github/workflows/docs-web.yml").read_text(encoding="utf-8")
        generate = workflow.index(
            "python python/build.py"
        )
        for consumer in (
            "unittest discover -s tools -p 'test_python_docs.py'",
            "python -m sphinx -W",
        ):
            self.assertLess(generate, workflow.index(consumer))


class PythonApiTypesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native_entries = read_stub(ROOT / "python/cosmol_viewer.pyi")
        # Exercise the shared declaration protocol even when a Viewer release
        # has no factory aliases, writable properties or overloaded methods.
        with tempfile.TemporaryDirectory() as folder:
            stub = Path(folder) / "sample.pyi"
            stub.write_text("""
import typing
class Sample:
    @staticmethod
    def create() -> Sample: ...
    @property
    def enabled(self) -> bool: ...
    @enabled.setter
    def enabled(self, value: bool) -> None: ...
    def find(self, index: int) -> Sample | None: ...
    @typing.overload
    def __getitem__(self, index: typing.SupportsIndex) -> Sample | None: ...
    @typing.overload
    def __getitem__(self, index: slice) -> list[Sample]: ...
create = Sample.create
""", encoding="utf-8")
            cls.entries = read_stub(stub)

    def test_factory_alias_reuses_method_types(self):
        self.assertEqual(self.entries["create"], self.entries["Sample.create"])
        self.assertEqual({type_text(n.returns) for n in self.entries["create"]}, {"Sample"})

    def test_property_getter_is_not_replaced_by_setter(self):
        entries = self.entries["Sample.enabled"]
        self.assertEqual(len(entries), 1)
        self.assertEqual(type_text(entries[0].returns), "bool")

    def test_existing_arguments_and_optional_results_are_preserved(self):
        app = SimpleNamespace(_python_api_types=self.entries)
        result = return_signature(app, "method", "cosmol_viewer.Sample.find", None, None, "(index)", None)
        self.assertEqual(result, ("(index)", "Sample | None"))
        self.assertIsNone(return_signature(app, "method", "other.Sample.find", None, None, "()", None))

    def test_overload_arguments_remain_paired_with_results(self):
        app = SimpleNamespace(_python_api_types=self.entries)
        signature, result = return_signature(app, "method", "cosmol_viewer.Sample.__getitem__", None, None, "(key)", None)
        self.assertIsNone(result)
        self.assertIn("index: typing.SupportsIndex", signature)
        self.assertIn("-> Sample | None", signature)
        self.assertIn("index: slice", signature)
        self.assertIn("-> list[Sample]", signature)
        self.assertNotIn("self", signature)

    def test_sphinx_renders_native_function_method_and_property_types(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "conf.py").write_text(
                f"import sys\nsys.path.insert(0, {str(DOCS)!r})\n"
                f"sys.path.insert(0, {str(ROOT / 'target/python-bindings/package')!r})\n"
                "import cosmol_viewer\n"
                "extensions = ['sphinx.ext.autodoc', 'api_types']\n"
                "exclude_patterns = ['html', 'doctrees']\n"
                f"python_api_stub = {str(ROOT / 'python/cosmol_viewer.pyi')!r}\n",
                encoding="utf-8",
            )
            (root / "index.rst").write_text(
                "API\n===\n\n.. autofunction:: cosmol_viewer.select\n\n"
                ".. automethod:: cosmol_viewer.Scene.add\n\n"
                ".. automethod:: cosmol_viewer.Scene.to_png\n\n"
                ".. autoattribute:: cosmol_viewer.Selection.ids\n",
                encoding="utf-8",
            )
            warnings = io.StringIO()
            app = Sphinx(str(root), str(root), str(root / "html"), str(root / "doctrees"),
                         "html", status=io.StringIO(), warning=warnings, freshenv=True,
                         warningiserror=True)
            app.build(force_all=True)
            self.assertEqual(app.statuscode, 0, warnings.getvalue())
            from sphinx import addnodes
            signatures = list(app.env.get_doctree("index").findall(addnodes.desc_signature))
            self.assertEqual(len(signatures), 4)
            for entry in signatures:
                self.assertTrue(list(entry.findall(addnodes.desc_name)), entry.astext())
            self.assertIn("-> Selector", signatures[0].astext())
            self.assertIn("style", signatures[1].astext())
            self.assertIn("None", signatures[1].astext())
            self.assertIn("-> bytes", signatures[2].astext())
            self.assertIn("width", signatures[2].astext())
            self.assertIn("list[int]", signatures[3].astext())


if __name__ == "__main__":
    unittest.main()
