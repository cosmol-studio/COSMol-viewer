"""Compare reusable molecule colors and restore the built-in palette."""
from pathlib import Path

from cosmolkit import Molecule
from cosmol_viewer import MoleculeColors, Scene, StickStyle, Viewer


def build_scene():
    ligand_file = Path(__file__).with_name("6fi1_ligand.sdf")
    ligand = Molecule.from_sdf(ligand_file.read_text(encoding="utf-8")).with_kekulized_bonds()
    xyz = ligand.coordinates_3d()
    cx, cy, cz = [sum(p[axis] for p in xyz) / len(xyz) for axis in range(3)]

    def place(x, y):
        # Arrange copies in a grid so their colors can be compared side by side.
        return ligand.with_3d_coordinates([
            [pz - cz + x, py - cy + y, cx - px] for px, py, pz in xyz
        ])

    reference = place(-5.5, 4.8)
    first = place(5.5, 4.8)
    second = place(-5.5, -4.8)
    different = place(5.5, -4.8)

    scene = Scene()
    scene.set_background_color("#021529")
    scene.set_camera_view(0, 0, 0, 39, [0, 0, 0], 32)

    # Top left uses the scene's carbon color. Other elements stay built-in.
    palette = MoleculeColors(elements={"C": "#5CCBE5"})
    scene.set_molecule_palette(palette)
    sticks = StickStyle()
    scene.set("palette", reference, sticks)

    # Top right and bottom left share one scheme. IDs are zero-based CK atom IDs.
    shared = palette.with_atom_colors({0: "#FFD166", 3: "#FFD166"})
    highlighted = StickStyle().colors(shared)
    scene.set("shared-a", first, highlighted)
    scene.set("shared-b", second, highlighted)

    # Bottom right uses a different color for the same atom IDs.
    pink = palette.with_atom_colors({0: "#F283CB", 3: "#F283CB"})
    scene.set("different", different, StickStyle().colors(pink))
    return scene, {
        "palette": reference,
        "shared-a": first,
        "shared-b": second,
        "different": different,
    }


def restore_colors(scene, molecules):
    # Reset the palette and replace existing displays to reproject their colors.
    scene.set_molecule_palette(MoleculeColors.default())
    sticks = StickStyle()
    for name, molecule in molecules.items():
        scene.set(name, molecule, sticks)


if __name__ == "__main__":
    scene, molecules = build_scene()
    directory = Path(__file__).resolve().parents[1]
    custom_png = directory / "molecule_colors.png"
    scene.save_image(str(custom_png), width=1200, height=900)
    print(f"Saved {custom_png}")

    # The viewer displays the custom snapshot while we export the reset scene.
    viewer = Viewer.show(scene, width=800, height=600)
    restore_colors(scene, molecules)
    restored_png = directory / "molecule_colors_builtin.png"
    scene.save_image(str(restored_png), width=1200, height=900)
    print(f"Saved {restored_png}")
    viewer.keep_alive()
