"""Regression examples for typed inputs, native ownership and foreign values."""
import sys
import unittest
from unittest.mock import patch

import cosmol_viewer as cv


class ProjectionTests(unittest.TestCase):
    def test_shape_inputs_snapshot_and_failed_replacement_is_atomic(self):
        scene = cv.Scene()
        sphere = cv.Sphere([0, 0, 0], 1)
        scene.set("atom", sphere)
        before = repr(scene)
        sphere.set_center([2, 3, 4])
        self.assertEqual(repr(scene), before)
        with self.assertRaises(TypeError):
            scene.set("atom", object())
        self.assertEqual(repr(scene), before)
        with self.assertRaises(ValueError):
            scene.set("atom", sphere, cv.Cartoon())
        self.assertEqual(repr(scene), before)
        scene.set("atom", sphere)
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


class ForeignMolecule:
    """Transport only: no molecule fields are exposed to the binding."""
    def __init__(self):
        import cosmolkit
        self.native = cosmolkit.Molecule.from_smiles("C").with_added_3d_conformer([[2, 3, 4]])
        self.imports = 0

    def to_transfer_binary(self):
        self.imports += 1
        return self.native.to_transfer_binary()


class ForeignProjectionTests(unittest.TestCase):
    def setUp(self):
        import cosmolkit
        self.source = ForeignMolecule()
        self.patch = patch.object(cosmolkit, "Molecule", ForeignMolecule)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def test_transport_only_import_is_shared_by_selection_and_scene(self):
        with self.assertRaises(TypeError): cv.select(object())
        source = self.source
        selected = cv.select(source).atoms()
        self.assertEqual(selected.ids, [0])
        scene = cv.Scene()
        scene.add(source)
        scene.recenter_on(source)
        scene.set("atom", source, cv.SphereStyle())
        scene.add(cv.select(source).atoms(), cv.SphereStyle())
        scene.set("atom", selected, cv.SphereStyle())
        del selected
        cv.select(source).atoms()
        self.assertEqual(source.imports, 1)
        del scene
        cv.select(source).atoms()
        self.assertEqual(source.imports, 2)

    def test_invalid_transfer_never_changes_scene_and_can_be_retried(self):
        source = self.source
        scene = cv.Scene()
        before = repr(scene)
        with patch.object(source, "to_transfer_binary", return_value=b"invalid transfer"):
            with self.assertRaises(ValueError): cv.select(source)
            with self.assertRaises(ValueError): scene.recenter_on(source)
        self.assertEqual(repr(scene), before)
        scene.add(cv.select(source).atoms(), cv.BallAndStick())

    def test_foreign_exception_identity_is_preserved(self):
        error = RuntimeError("foreign transfer failed")
        with patch.object(self.source, "to_transfer_binary", side_effect=error):
            with self.assertRaises(RuntimeError) as caught: cv.select(self.source)
        self.assertIs(caught.exception, error)

    def test_nonzero_conformer_id_uses_native_selection(self):
        source = self.source
        source.native = source.native.with_added_3d_conformer([[4, 5, 6]])
        conformer_id = source.native.conformers_3d[-1].id
        self.assertNotEqual(conformer_id, 0)
        scene = cv.Scene()
        selected = cv.select(source).atoms()
        with self.assertRaisesRegex(ValueError, "conformers"):
            scene.add(selected, cv.BallAndStick())
        scene.add(selected, cv.BallAndStick().conformer(conformer_id))
        self.assertEqual(source.imports, 1)


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
        scene.set("mol", all_atoms, cv.BallAndStick())
        saved = repr(scene)
        scene.add(oxygen, cv.SphereStyle().scale(1.4))
        self.assertNotEqual(repr(scene), saved)
        scene.recenter_on(source)
        scene.set("mol", oxygen, cv.SphereStyle())
        self.assertEqual(source.to_binary(), before)
        self.assertFalse(hasattr(cv, "Molecule"))
        self.assertFalse(hasattr(cv, "Protein"))

    def test_default_styles_and_shapes_share_add_and_set(self):
        source = self.molecule()
        defaults, explicit = cv.Scene(), cv.Scene()
        defaults.set("item", source)
        explicit.set("item", source, cv.BallAndStick())
        self.assertEqual(repr(defaults), repr(explicit))
        defaults.set("item", cv.Sphere([1, 2, 3], 0.5))
        defaults.add(cv.Stick([0, 0, 0], [1, 1, 1], 0.2))
        before = repr(defaults)
        with self.assertRaises(ValueError): defaults.set("item", cv.Sphere([0, 0, 0], 1), cv.Cartoon())
        self.assertEqual(repr(defaults), before)
        defaults.set("new", source)
        self.assertNotEqual(repr(defaults), before)
        for name in ["add_shape", "add_shape_with_id", "replace_shape"]:
            self.assertFalse(hasattr(defaults, name))

    def test_missing_coordinates_and_invalid_style_are_atomic(self):
        source = self.molecule()
        selected = cv.select(source).atoms()
        scene = cv.Scene()
        scene.set("mol", selected, cv.BallAndStick())
        before = repr(scene)
        with self.assertRaises(ValueError):
            scene.set("mol", selected, cv.Cartoon())
        self.assertEqual(repr(scene), before)
        source.set_atom_position_(0, [9, 9, 9])
        # This Scene owns the first import; the original selection still draws
        # that snapshot even if the Python source has subsequently changed.
        scene.set("mol", selected, cv.BallAndStick())
        self.assertEqual(repr(scene), before)
        scene.set("mol", cv.select(source).atoms(), cv.BallAndStick())
        self.assertEqual(repr(scene), before)
        changed = source.copy()
        scene.set("mol", cv.select(changed).atoms(), cv.BallAndStick())
        self.assertNotEqual(repr(scene), before)
        unpositioned = self.ck.Molecule.from_smiles("CO")
        archive = unpositioned.to_binary()
        with self.assertRaisesRegex(ValueError, "no stored coordinates"):
            scene.add(cv.select(unpositioned).atoms(), cv.BallAndStick())
        self.assertEqual(unpositioned.to_binary(), archive)
        cv.Scene().add(cv.select(source).atoms(), cv.BallAndStick())

    def test_aromatic_bonds_and_invalid_kind_or_style(self):
        source = self.ck.Molecule.from_smiles("c1ccccc1").with_2d_coordinates()
        aromatic = cv.select(source).bonds().aromatic()
        self.assertEqual(len(aromatic), 6)
        scene = cv.Scene()
        scene.add(aromatic, cv.StickStyle())
        before = repr(scene)
        with self.assertRaises(ValueError): scene.add(aromatic, cv.BallAndStick())
        with self.assertRaises(ValueError): cv.select(source).residues()
        with self.assertRaises(ValueError): aromatic.element(self.ck.Element.C)
        with self.assertRaises(TypeError): cv.select(source).atoms().element(8)
        self.assertEqual(repr(scene), before)

    def test_multiple_conformers_require_explicit_id(self):
        source = self.molecule().with_added_3d_conformer([[1, 0, 1], [3, 0, 3]])
        atoms = cv.select(source).atoms()
        scene = cv.Scene()
        with self.assertRaisesRegex(ValueError, "conformers"): scene.add(atoms, cv.BallAndStick())
        scene.add(atoms, cv.BallAndStick().conformer(source.conformers_3d[-1].id))

    def test_protein_and_ligand_use_ck_objects_directly(self):
        from pathlib import Path
        root = Path(__file__).resolve().parents[2] / "cosmol_viewer/examples"
        protein = self.ck.Protein.from_mmcif((root / "6fi1.cif").read_text(encoding="utf-8"))
        ligand = self.ck.Molecule.from_sdf((root / "6fi1_ligand.sdf").read_text(encoding="utf-8"))
        residues = cv.select(protein).residues()
        scene = cv.Scene()
        scene.set("protein", residues, cv.Cartoon().rainbow_residues())
        scene.set("ligand", cv.select(ligand).atoms(), cv.BallAndStick())
        scene.add(cv.select(protein).atoms().element(self.ck.Element.O), cv.SphereStyle())
        scene.recenter_on(ligand)
        before = repr(scene)
        with self.assertRaises(ValueError): scene.add(residues, cv.BallAndStick())
        protein.translate_([1, 0, 0])
        scene.set("protein", residues, cv.Cartoon().rainbow_residues())
        self.assertEqual(repr(scene), before)

    def test_protein_transport_only_then_native_selection_and_scene(self):
        from pathlib import Path
        path = Path(__file__).resolve().parents[2] / "cosmol_viewer/examples/6fi1.cif"
        protein = self.ck.Protein.from_mmcif(path.read_text(encoding="utf-8"))
        archive = protein.to_transfer_binary()

        class Transport:
            imports = 0
            def to_transfer_binary(self):
                self.imports += 1
                return protein.to_transfer_binary()

        source = Transport()
        with patch.object(self.ck, "Protein", Transport):
            scene = cv.Scene()
            before = repr(scene)
            error = RuntimeError("protein transfer failed")
            with patch.object(source, "to_transfer_binary", side_effect=error):
                with self.assertRaises(RuntimeError) as caught:
                    cv.select(source)
                self.assertIs(caught.exception, error)
            with patch.object(source, "to_transfer_binary", return_value=b"invalid protein transfer"):
                with self.assertRaises(ValueError):
                    scene.recenter_on(source)
            self.assertEqual(repr(scene), before)
            scene.set("protein", source)
            scene.add(source, cv.Surface().configure(grid_spacing=1.5))
            selector = cv.select(source)
            atoms = selector.atoms()
            residues = selector.residues()
            self.assertEqual(atoms.ids, [atom.id for atom in protein.atoms])
            self.assertEqual(residues.ids, [residue.id for residue in protein.residues])
            oxygen = atoms.element(self.ck.Element.O)
            self.assertEqual(oxygen.ids, [a.id for a in protein.atoms if a.element == self.ck.Element.O])
            scene.set("protein", residues, cv.Cartoon())
            scene.add(oxygen, cv.SphereStyle())
            self.assertEqual(source.imports, 1)
            scene.recenter_on(source)
            before = repr(scene)
            with self.assertRaises(ValueError):
                scene.set("protein", residues, cv.BallAndStick())
            self.assertEqual(repr(scene), before)
            self.assertEqual(protein.to_transfer_binary(), archive)
            protein.translate_([10, 0, 0])
            scene.set("protein", cv.select(source).residues(), cv.Cartoon())
            scene.recenter_on(source)
            self.assertEqual(repr(scene), before)
            self.assertEqual(source.imports, 1)

    def test_pdb_protein_uses_binary_native_import(self):
        text = "ATOM      1  CA  ALA A   1       1.000   2.000   3.000  1.00 20.00           C  \n"
        source = self.ck.Protein.from_pdb(text)
        scene = cv.Scene()
        selected = cv.select(source).atoms()
        self.assertEqual(len(selected), 1)
        scene.add(selected, cv.SphereStyle())
        scene.recenter_on(source)

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
