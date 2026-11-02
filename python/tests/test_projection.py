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
    def __init__(self, rows): self.rows = rows
    def tolist(self): return self.rows

class ForeignMolecule:
    atoms = [types.SimpleNamespace(atomic_number=6)]
    bonds = []
    conformers_3d = []
    coordinates_2d = Array([[2.0, 3.0]])
    properties = types.SimpleNamespace(sdf_data_fields=[])
    def atom_property_double(self, index, name): return None

class ForeignProjectionTests(unittest.TestCase):
    def setUp(self):
        module = types.ModuleType("cosmolkit")
        module.Molecule = ForeignMolecule
        self.modules = patch.dict(sys.modules, {"cosmolkit": module})
        self.modules.start()
        self.addCleanup(self.modules.stop)

    def test_nominal_external_type_and_property_projection(self):
        with self.assertRaises(TypeError): cv.select(object())
        source = ForeignMolecule()
        selected = cv.select(source).atoms()
        self.assertEqual(selected.ids, [0])
        scene = cv.Scene()
        scene.show(source, selected, cv.BallAndStick())
        scene.recenter_on(source)

    def test_invalid_foreign_coordinates_never_change_scene(self):
        for rows in [[], [[1, 2], [3, 4]], [[1]], [[1, 2, 3]], [[float("inf"), 2]], [[1e100, 2]]]:
            source = ForeignMolecule()
            source.coordinates_2d = Array(rows)
            scene = cv.Scene()
            before = repr(scene)
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                scene.show(source, cv.select(source).atoms(), cv.BallAndStick())
            self.assertEqual(repr(scene), before)

    def test_foreign_exception_identity_is_preserved(self):
        source = ForeignMolecule()
        error = RuntimeError("foreign read failed")
        def fail(*args): raise error
        source.atom_property_double = fail
        with self.assertRaises(RuntimeError) as caught: cv.select(source)
        self.assertIs(caught.exception, error)

    def test_first_stored_conformer_does_not_assume_id_zero(self):
        source = ForeignMolecule()
        source.conformers_3d = [types.SimpleNamespace(id=8, coordinates=Array([[4, 5, 6]]))]
        scene = cv.Scene()
        scene.show(source, cv.select(source).atoms(), cv.BallAndStick().conformer(8))
        scene.recenter_on(source)


class InstalledCosmolkitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import cosmolkit
        cls.ck = cosmolkit

    def molecule(self):
        return self.ck.Molecule.from_smiles("CO").with_added_3d_conformer([[0, 0, 1], [2, 0, 3]])

    def test_real_source_selection_multiple_styles_and_named_replacement(self):
        ck = self.ck
        source = self.molecule()
        before = source.to_binary()
        all_atoms = cv.select(source).atoms()
        oxygen = all_atoms.element(ck.Element.O)
        self.assertEqual(oxygen.ids, [1])
        self.assertEqual(len(all_atoms), 2)
        scene = cv.Scene()
        scene.show_with_id("mol", source, all_atoms, cv.BallAndStick())
        saved = repr(scene)
        scene.show(source, oxygen, cv.SphereStyle().scale(1.4))
        self.assertNotEqual(repr(scene), saved)
        scene.recenter_on(source)
        scene.show_with_id("mol", source, oxygen, cv.SphereStyle())
        self.assertEqual(source.to_binary(), before)
        self.assertFalse(hasattr(cv, "Molecule"))
        self.assertFalse(hasattr(cv, "Protein"))

    def test_missing_coordinates_and_source_mismatch_are_atomic(self):
        source = self.molecule()
        selected = cv.select(source).atoms()
        scene = cv.Scene()
        scene.show_with_id("mol", source, selected, cv.BallAndStick())
        before = repr(scene)
        with self.assertRaisesRegex(ValueError, "different source"):
            scene.show_with_id("mol", source.copy(), selected, cv.BallAndStick())
        self.assertEqual(repr(scene), before)
        source.set_atom_position_(0, [9, 9, 9])
        with self.assertRaisesRegex(ValueError, "changed"):
            scene.show_with_id("mol", source, selected, cv.BallAndStick())
        self.assertEqual(repr(scene), before)
        unpositioned = self.ck.Molecule.from_smiles("CO")
        archive = unpositioned.to_binary()
        with self.assertRaisesRegex(ValueError, "no stored coordinates"):
            scene.show(unpositioned, cv.select(unpositioned).atoms(), cv.BallAndStick())
        self.assertEqual(unpositioned.to_binary(), archive)
        scene.show(source, cv.select(source).atoms(), cv.BallAndStick())

    def test_aromatic_bonds_and_invalid_kind_or_style(self):
        source = self.ck.Molecule.from_smiles("c1ccccc1").with_2d_coordinates()
        aromatic = cv.select(source).bonds().aromatic()
        self.assertEqual(len(aromatic), 6)
        scene = cv.Scene()
        scene.show(source, aromatic, cv.StickStyle())
        before = repr(scene)
        with self.assertRaises(ValueError): scene.show(source, aromatic, cv.BallAndStick())
        with self.assertRaises(ValueError): cv.select(source).residues()
        with self.assertRaises(ValueError): aromatic.element(self.ck.Element.C)
        with self.assertRaises(TypeError): cv.select(source).atoms().element(8)
        self.assertEqual(repr(scene), before)

    def test_multiple_conformers_require_explicit_id(self):
        source = self.molecule().with_added_3d_conformer([[1, 0, 1], [3, 0, 3]])
        atoms = cv.select(source).atoms()
        scene = cv.Scene()
        with self.assertRaisesRegex(ValueError, "conformers"): scene.show(source, atoms, cv.BallAndStick())
        scene.show(source, atoms, cv.BallAndStick().conformer(source.conformers_3d[-1].id))

    def test_protein_and_ligand_use_ck_objects_directly(self):
        from pathlib import Path
        root = Path(__file__).resolve().parents[2] / "cosmol_viewer/examples"
        protein = self.ck.Protein.from_mmcif((root / "6fi1.cif").read_text(encoding="utf-8"))
        ligand = self.ck.Molecule.from_sdf((root / "6fi1_ligand.sdf").read_text(encoding="utf-8"))
        residues = cv.select(protein).residues()
        scene = cv.Scene()
        scene.show_with_id("protein", protein, residues, cv.Cartoon().rainbow_residues())
        scene.show_with_id("ligand", ligand, cv.select(ligand).atoms(), cv.BallAndStick())
        scene.show(protein, cv.select(protein).atoms().element(self.ck.Element.O), cv.SphereStyle())
        scene.recenter_on(ligand)
        before = repr(scene)
        with self.assertRaises(ValueError): scene.show(protein, residues, cv.BallAndStick())
        protein.translate_([1, 0, 0])
        with self.assertRaisesRegex(ValueError, "changed"): scene.show(protein, residues, cv.Cartoon())
        self.assertEqual(repr(scene), before)

    def test_style_validation_does_not_panic_or_mutate_on_error(self):
        style = cv.SphereStyle()
        for value in [0, -1, float("nan"), float("inf")]:
            with self.subTest(value=value), self.assertRaises(ValueError): style.scale(value)
        for value in [-1, 2, float("nan")]:
            with self.assertRaises(ValueError): style.opacity(value)
        self.assertIs(style.color("#123456").scale(1.4), style)
        with self.assertRaises(ValueError): cv.Surface().configure(-1, .5, False, True)


if __name__ == "__main__":
    unittest.main()
