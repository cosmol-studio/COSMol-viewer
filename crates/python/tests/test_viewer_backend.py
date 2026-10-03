"""Backend regression checks without opening a GUI or requiring a notebook browser.

Run after installing the extension:
    python crates/python/tests/test_viewer_backend.py
"""

import contextlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import types
import unittest


def verify_backend(mode):
    # Runtime detection is cached, so each environment needs its own process.
    ipython = types.ModuleType("IPython")
    display_module = types.ModuleType("IPython.display")
    displays = []

    class DisplayObject:
        def __init__(self, data):
            self.data = data

    display_module.HTML = DisplayObject
    display_module.Javascript = DisplayObject
    display_module.display = displays.append
    shell_name = {
        "colab": "ZMQInteractiveShell",
        "jupyter": "ZMQInteractiveShell",
        "terminal": "TerminalInteractiveShell",
        "unsupported": "OtherShell",
    }.get(mode)
    shell = type(shell_name, (), {})() if shell_name else None
    ipython.get_ipython = lambda: shell
    ipython.display = display_module
    sys.modules["IPython"] = ipython
    sys.modules["IPython.display"] = display_module
    if mode == "colab":
        sys.modules["google.colab"] = types.ModuleType("google.colab")

    from cosmol_viewer import Animation, Protein, Scene, Viewer

    assert callable(Viewer.keep_alive)

    scene = Scene()
    # Exercise the temporary CK residue-name Serde adapter in actual WASM payloads.
    scene.add_shape(Protein.from_pdb("""\
ATOM      1  N   MSE A   1      11.104  13.207   9.900  1.00 20.00           N
ATOM      2  CA  MSE A   1      12.210  13.912  10.555  1.00 20.00           C
ATOM      3  C   MSE A   1      13.470  13.079  10.413  1.00 20.00           C
ATOM      4  O   MSE A   1      14.000  12.500  11.000  1.00 20.00           O
END
"""))
    animation = Animation(interval=0.1, loops=1, interpolate=False)
    animation.add_frame(scene)

    if mode in {"plain", "terminal", "unsupported"}:
        # Missing renderer errors must stay recoverable and must not open a GUI.
        os.environ["COSMOL_VIEWER_NATIVE_PATH"] = str(Path(__file__).with_name("missing-native-viewer"))
        for create, item in ((Viewer.render, scene), (Viewer.play, animation)):
            try:
                create(item, 100.0, 100.0)
            except (RuntimeError, ValueError) as error:
                expected = "runtime environment" if mode == "unsupported" else "Native viewer executable is missing"
                assert expected in str(error), str(error)
                assert "NotRegistered" not in str(error)
            else:
                raise AssertionError("A failed backend creation returned a Viewer")
        return

    assert Viewer.get_environment() == {"colab": "Colab", "jupyter": "Jupyter"}[mode]
    viewer = Viewer.render(scene, 100.0, 100.0)
    assert any("start_with_scene" in item.data for item in displays)
    display_count = len(displays)
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        try:
            viewer.keep_alive()
        except RuntimeError as error:
            assert "not Jupyter or Colab" in str(error), str(error)
        else:
            raise AssertionError("Notebook keep_alive() did not reject the call")
    assert output.getvalue() == ""
    assert len(displays) == display_count
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        viewer.update(scene)
        viewer.update(scene)
    assert output.getvalue().count("Note: When running in Jupyter or Colab") == 1
    assert sum("app.dispatch(" in item.data for item in displays) == 2
    assert not any("app.update_scene(" in item.data for item in displays)

    viewer.set_camera_parameter_logging()
    viewer.set_camera_parameter_logging(False)
    assert sum("app.dispatch(" in item.data for item in displays) == 4

    playing = Viewer.play(animation, 100.0, 100.0)
    assert any("initiate_viewer_and_play" in item.data for item in displays)
    try:
        playing.keep_alive()
    except RuntimeError as error:
        assert "not Jupyter or Colab" in str(error), str(error)
    else:
        raise AssertionError("Notebook animation keep_alive() did not reject the call")
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        playing.update(scene)
    assert "Note: When running in Jupyter or Colab" not in output.getvalue()

    # Feed actual Python-generated commands into the compiled WASM receiver.
    payloads = [
        json.loads(match.group(1))
        for item in displays
        if (match := re.search(r"await app\.dispatch\((.+)\);", item.data))
    ]
    assert len(payloads) == 5
    verify_wasm_receiver(payloads)

    def fail_display(item):
        raise ValueError("display failure")

    display_module.display = fail_display
    try:
        playing.update(scene)
    except ValueError as error:
        assert str(error) == "display failure"
    else:
        raise AssertionError("Notebook update swallowed the bridge error")


def verify_wasm_receiver(payloads):
    node = shutil.which("node")
    if node is None:
        return
    pkg = Path(__file__).resolve().parents[2] / "wasm" / "pkg"
    script = """
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const mod = await import(pathToFileURL(process.argv[1]).href);
await mod.default({ module_or_path: readFileSync(process.argv[2]) });
const app = new mod.WebHandle();
for (const payload of process.argv.slice(3)) {
    assert.throws(
        () => app.dispatch(payload),
        error => String(error).includes('before app initialization'),
    );
}
app.free();
"""
    result = subprocess.run(
        [node, "--input-type=module", "-e", script,
         str(pkg / "cosmol_viewer_wasm.js"),
         str(pkg / "cosmol_viewer_wasm_bg.wasm"), *payloads],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=20,
    )
    assert result.returncode == 0, result.stdout + result.stderr


class ViewerBackendTests(unittest.TestCase):
    def test_backends_in_separate_processes(self):
        for mode in ("colab", "jupyter", "plain", "terminal", "unsupported"):
            with self.subTest(environment=mode):
                result = subprocess.run(
                    [sys.executable, str(Path(__file__).resolve()), mode],
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    timeout=30,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    if len(sys.argv) == 2:
        verify_backend(sys.argv[1])
    else:
        unittest.main()
