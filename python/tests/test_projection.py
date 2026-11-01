"""Regression examples for typed inputs, native ownership and foreign values."""
import sys
import types
import unittest
from unittest.mock import patch

import cosmol_viewer as cv


class ProjectionTests(unittest.TestCase):
    def test_shape_inputs_snapshot_and_failed_replacement_is_atomic(self):
        scene = cv.Scene()
        sphere = cv.Sphere([0, 0, 0], 1)
        scene.add_shape_with_id("atom", sphere)
        before = repr(scene)
        sphere.set_center([2, 3, 4])
        self.assertEqual(repr(scene), before)
        with self.assertRaises(TypeError):
            scene.replace_shape("atom", object())
        self.assertEqual(repr(scene), before)
        with self.assertRaises(KeyError):
            scene.replace_shape("missing", sphere)
        self.assertEqual(repr(scene), before)
        scene.replace_shape("atom", sphere)
        self.assertNotEqual(repr(scene), before)

    def test_color_conversion_never_panics_and_preserves_scene_on_error(self):
        scene = cv.Scene()
        scene.set_background_color([255, 0, 0])
        before = repr(scene)
        for color in ["bad", "#gg0000", "ééé", [256, 0, 0], [-1, 0, 0]]:
            with self.subTest(color=color), self.assertRaises(ValueError):
                scene.set_background_color(color)
            self.assertEqual(repr(scene), before)
        for color in [object(), [1, 2], [0.5, 0, 0]]:
            with self.subTest(color=color), self.assertRaises(TypeError):
                scene.set_background_color(color)
            self.assertEqual(repr(scene), before)
        sphere = cv.Sphere([0, 0, 0], 1)
        self.assertIs(sphere.color("#ffffff"), sphere)
        self.assertIs(sphere.color((0, 0, 0)), sphere)

    def test_animation_indexes_and_returns_independent_scene_snapshots(self):
        scene = cv.Scene()
        animation = cv.Animation(0.1, 1, False)
        with self.assertRaises(IndexError):
            animation[-sys.maxsize - 1]
        animation.add_frame(scene)
        saved = repr(animation[0])
        scene.set_scale(2)
        copy = animation[-1]
        copy.set_scale(3)
        self.assertEqual(repr(animation[0]), saved)
        self.assertEqual(len(animation), 1)
        for index in [1, -2, -sys.maxsize - 1]:
            with self.subTest(index=index), self.assertRaises(IndexError):
                animation[index]

    def test_invalid_background_is_rejected_before_starting_renderer(self):
        scene = cv.Scene()
        for background in ["ééé", "#gg000000", [1, 2, 3, 256], [0.0, 0.0, 0.0, float("nan")]]:
            with self.subTest(background=background), self.assertRaises(ValueError):
                scene.to_png(background=background)


class Array:
    def __init__(self, rows):
        self.rows = rows

    def tolist(self):
        return self.rows


class ForeignMolecule:
    atoms = [types.SimpleNamespace(atomic_number=6)]
    bonds = []
    conformers_3d = []
    coordinates_2d = Array([[2.0, 3.0]])
    layout_calls = 0

    def with_kekulized_bonds(self):
        return self

    def with_2d_coordinates(self):
        self.layout_calls += 1
        return self


class ForeignProjectionTests(unittest.TestCase):
    def setUp(self):
        module = types.ModuleType("cosmolkit")
        module.Molecule = ForeignMolecule
        self.modules = patch.dict(sys.modules, {"cosmolkit": module})
        self.modules.start()
        self.addCleanup(self.modules.stop)

    def test_nominal_external_type_and_property_projection(self):
        with self.assertRaises(TypeError):
            cv.Molecule.from_cosmolkit(object())
        molecule = cv.Molecule.from_cosmolkit(ForeignMolecule())
        self.assertEqual(list(molecule.get_center()), [2.0, 3.0, 0.0])

    def test_invalid_foreign_coordinates_never_trigger_layout_fallback(self):
        for rows in [[], [[1, 2], [3, 4]], [[1]], [[1, 2, 3]], [[float("inf"), 2]], [[1e100, 2]]]:
            source = ForeignMolecule()
            source.coordinates_2d = Array(rows)
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                cv.Molecule.from_cosmolkit(source)
            self.assertEqual(source.layout_calls, 0)

    def test_foreign_exception_identity_is_preserved(self):
        source = ForeignMolecule()
        error = RuntimeError("foreign chemistry failed")

        def fail():
            raise error

        source.with_kekulized_bonds = fail
        with self.assertRaises(RuntimeError) as caught:
            cv.Molecule.from_cosmolkit(source)
        self.assertIs(caught.exception, error)
        self.assertEqual(source.layout_calls, 0)

    def test_first_stored_conformer_does_not_assume_id_zero(self):
        source = ForeignMolecule()
        source.conformers_3d = [types.SimpleNamespace(id=8, coordinates=Array([[4, 5, 6]]))]
        converted = cv.Molecule.from_cosmolkit(source)
        self.assertEqual(list(converted.get_center()), [4.0, 5.0, 6.0])


class InstalledCosmolkitTests(unittest.TestCase):
    def test_real_molecule_with_and_without_stored_coordinates(self):
        try:
            import cosmolkit as ck
        except ImportError:
            self.skipTest("Optional COSMolKit package is not installed")
        source = ck.mol_from_smiles("CCO")
        before = source.to_binary()
        self.assertIsInstance(cv.Molecule.from_cosmolkit(source), cv.Molecule)
        self.assertEqual(source.to_binary(), before)
        positioned = source.with_added_3d_conformer([[0, 0, 1], [1, 0, 2], [2, 0, 3]])
        self.assertEqual(list(cv.Molecule.from_cosmolkit(positioned).get_center()), [1.0, 0.0, 2.0])


if __name__ == "__main__":
    unittest.main()
