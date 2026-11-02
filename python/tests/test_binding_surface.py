"""Validate registered PyO3 APIs and their generated (never hand-edited) stubs."""

import ast
import inspect
import json
from pathlib import Path
import unittest

import cosmol_viewer as cv


class BindingSurfaceTests(unittest.TestCase):
    def test_every_public_runtime_member_has_an_official_declaration(self):
        stub = ast.parse(Path(__file__).resolve().parents[1].joinpath(
            "cosmol_viewer.pyi").read_text(encoding="utf-8"))
        declarations = {node.name: node for node in stub.body
                        if isinstance(node, (ast.ClassDef, ast.FunctionDef))}
        for name in dir(cv):
            if name.startswith("_"):
                continue
            value = getattr(cv, name)
            if not callable(value):
                continue
            with self.subTest(export=name):
                self.assertIn(name, declarations)
                if isinstance(value, type):
                    methods = {node.name for node in declarations[name].body
                               if isinstance(node, ast.FunctionDef)}
                    for member in vars(value):
                        if not member.startswith("_") and callable(getattr(value, member)):
                            self.assertIn(member, methods)
                    self.assertEqual(value.__module__, "cosmol_viewer")
        unknown = {node.id for node in ast.walk(stub) if isinstance(node, ast.Name)} & {"Any", "Incomplete"}
        self.assertFalse(unknown)

    def test_registered_python_api_matches_runtime_and_stub(self):
        contract = json.loads(cv.binding_contract_json())
        stub = ast.parse(Path(__file__).resolve().parents[1].joinpath(
            "cosmol_viewer.pyi").read_text(encoding="utf-8"))
        classes = {node.name: node for node in stub.body if isinstance(node, ast.ClassDef)}
        functions = {node.name for node in stub.body if isinstance(node, ast.FunctionDef)}
        self.assertIn("binding_contract_json", functions)
        self.assertEqual(len(contract), len({row["semantic_id"] for row in contract}))
        for row in contract:
            projection = row["python"]
            name = projection["name"]
            if name is None:
                self.assertTrue(projection["unsupported_reason"])
                continue
            with self.subTest(api=name):
                owner, _, method = name.partition(".")
                runtime = getattr(cv, owner)
                self.assertIn(owner, classes if method or isinstance(runtime, type) else functions)
                if method:
                    runtime = getattr(runtime, method)
                    methods = {node.name for node in classes[owner].body
                               if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))}
                    self.assertIn(method, methods)
                self.assertTrue(callable(runtime))
                for default in row["defaults"]:
                    signature = inspect.signature(runtime)
                    self.assertEqual(signature.parameters[default["parameter"]].default,
                                     ast.literal_eval(default["python"]))

    def test_native_wait_is_explicitly_unavailable_in_browser_and_notebook(self):
        rows = {row["semantic_id"]: row for row in json.loads(cv.binding_contract_json())}
        wait = rows["Viewer.keep_alive"]
        self.assertEqual(wait["rust_platforms"], ["native"])
        self.assertEqual(wait["python"]["platforms"], ["native"])
        self.assertIsNone(wait["javascript"]["name"])
        self.assertTrue(wait["javascript"]["unsupported_reason"])
        status = rows["Viewer.is_open"]
        self.assertEqual(status["python"]["platforms"], ["native"])
        self.assertIsNone(status["javascript"]["name"])


if __name__ == "__main__":
    unittest.main()
