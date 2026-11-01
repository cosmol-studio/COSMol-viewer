"""Fixed infrastructure checks; these fixtures deliberately have no viewer APIs."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HERE))
from framework.build import extract_stubs
import build_backend


class ExtractionTests(unittest.TestCase):
    def extract(self, declaration, destination):
        def extractor(arguments, **_):
            Path(arguments[3], "__init__.pyi").write_text(declaration, encoding="utf-8")

        with patch("framework.build.subprocess.run", side_effect=extractor):
            extract_stubs(Path("example.dll"), "unrelated", "extractor", destination)

    def test_new_types_and_calls_are_preserved_verbatim(self):
        declaration = "class Unrelated:\n    def query(self, value: tuple[int, str]) -> bytes: ...\n"
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary, "module.pyi")
            self.extract(declaration, destination)
            self.assertEqual(destination.read_text(encoding="utf-8"), declaration)

    def test_incomplete_extraction_never_replaces_previous_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary, "module.pyi")
            destination.write_text("previous", encoding="utf-8")
            for type_name in ["Any", "Incomplete"]:
                with self.subTest(type_name=type_name), self.assertRaises(ValueError):
                    self.extract(f"def query(value: {type_name}) -> None: ...\n", destination)
                self.assertEqual(destination.read_text(encoding="utf-8"), "previous")


class PackagingTests(unittest.TestCase):
    def test_pep517_preserves_options_and_always_generates_types(self):
        settings = {"maturin.build-args": ["--target", "example-target", "--release"]}
        actual = build_backend._with_generated_stubs(settings)
        self.assertEqual(actual["maturin.build-args"], ["--target", "example-target", "--release", "--generate-stubs"])
        self.assertEqual(settings["maturin.build-args"], ["--target", "example-target", "--release"])
        self.assertEqual(build_backend._with_generated_stubs(actual), actual)


if __name__ == "__main__":
    unittest.main()
