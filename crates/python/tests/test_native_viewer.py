"""Opt-in native GUI integration tests: python test_native_viewer.py --gui."""

import ctypes
import os
from pathlib import Path
import queue
import re
import signal
import subprocess
import sys
import threading
import time
import unittest


def run_viewer(mode):
    from cosmol_viewer import Animation, Protein, Scene, Sphere, Viewer

    scene = Scene()
    scene.add_shape_with_id("sphere", Sphere([0.0, 0.0, 0.0], 1.0))
    # Native IPC must preserve modified CK residue codes through render and update.
    scene.add_shape_with_id("protein", Protein.from_pdb("""\
ATOM      1  N   HYP A   1      11.104  13.207   9.900  1.00 20.00           N
ATOM      2  CA  HYP A   1      12.210  13.912  10.555  1.00 20.00           C
ATOM      3  C   HYP A   1      13.470  13.079  10.413  1.00 20.00           C
ATOM      4  O   HYP A   1      14.000  12.500  11.000  1.00 20.00           O
END
"""))
    if mode == "play":
        animation = Animation(0.05, -1, False)
        animation.add_frame(scene)
        viewer = Viewer.play(animation, 100.0, 100.0)
    else:
        viewer = Viewer.render(scene, 100.0, 100.0)
    # Let a static renderer become idle; IPC commands must wake it without polling.
    time.sleep(0.2)
    viewer.update(scene)
    viewer.camera_parameter_logging(False)
    viewer.show_fps()
    viewer.show_camera_parameters()
    viewer.show_fps(False)
    viewer.show_camera_parameters(False)
    print("READY", flush=True)
    assert viewer.is_open()
    if mode.startswith("dynamic_"):
        while viewer.is_open():
            if mode != "dynamic_idle":
                viewer.update(scene)
            time.sleep(0.005)
        assert not viewer.is_open()
        print("DONE", flush=True)
        return
    if mode == "interrupt":
        import _thread

        threading.Timer(0.1, _thread.interrupt_main).start()
    try:
        viewer.keep_alive()
    except KeyboardInterrupt:
        assert mode == "interrupt"
        print("INTERRUPTED", flush=True)
    except OSError:
        assert mode == "failure"
        print("CHILD_FAILED", flush=True)
    else:
        assert mode not in {"interrupt", "failure"}
    assert not viewer.is_open()
    for operation in (viewer.keep_alive, lambda: viewer.update(scene),
                      lambda: viewer.camera_parameter_logging(),
                      viewer.show_fps, viewer.show_camera_parameters):
        try:
            operation()
        except RuntimeError as error:
            assert "finalized" in str(error), str(error)
        else:
            raise AssertionError("A finalized viewer was reused")
    print("DONE", flush=True)


class ChildObserver:
    def __init__(self, pid):
        self.pid = pid
        if os.name == "nt":
            self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
            self.kernel.OpenProcess.argtypes = [ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
            self.kernel.OpenProcess.restype = ctypes.c_void_p
            self.kernel.GetExitCodeProcess.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
            self.kernel.TerminateProcess.argtypes = [ctypes.c_void_p, ctypes.c_uint]
            self.kernel.CloseHandle.argtypes = [ctypes.c_void_p]
            self.handle = self.kernel.OpenProcess(0x00100000 | 0x1000 | 0x0001, False, pid)
            if not self.handle:
                raise ctypes.WinError(ctypes.get_last_error())

    def is_running(self):
        if os.name == "nt":
            code = ctypes.c_ulong()
            if not self.kernel.GetExitCodeProcess(self.handle, ctypes.byref(code)):
                raise ctypes.WinError(ctypes.get_last_error())
            return code.value == 259  # STILL_ACTIVE
        try:
            os.kill(self.pid, 0)
        except ProcessLookupError:
            return False
        return True

    def terminate(self):
        if os.name == "nt":
            if not self.kernel.TerminateProcess(self.handle, 7):
                raise ctypes.WinError(ctypes.get_last_error())
        else:
            os.kill(self.pid, signal.SIGKILL)

    def close_window(self):
        user = ctypes.WinDLL("user32", use_last_error=True)
        callback_type = ctypes.WINFUNCTYPE(ctypes.c_int, ctypes.c_void_p, ctypes.c_void_p)
        user.EnumWindows.argtypes = [callback_type, ctypes.c_void_p]
        user.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
        user.PostMessageW.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_size_t, ctypes.c_ssize_t]
        windows = []

        @callback_type
        def visit(window, _):
            pid = ctypes.c_ulong()
            user.GetWindowThreadProcessId(window, ctypes.byref(pid))
            if pid.value == self.pid:
                windows.append(window)
            return True

        user.EnumWindows(visit, None)
        assert windows, "No renderer window found"
        for window in windows:
            assert user.PostMessageW(window, 0x0010, 0, 0)  # WM_CLOSE

    def close(self):
        if os.name == "nt":
            self.kernel.CloseHandle(self.handle)


class NativeViewerTests(unittest.TestCase):
    def exercise(self, mode):
        messages = queue.Queue()
        output = []
        process = subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "--child-test", mode],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace",
        )

        def read(stream):
            for line in stream:
                output.append(line)
                messages.put(line)

        readers = [threading.Thread(target=read, args=(stream,), daemon=True)
                   for stream in (process.stdout, process.stderr)]
        for reader in readers:
            reader.start()
        child = None
        try:
            ready = False
            deadline = time.monotonic() + 30
            while child is None or not ready:
                try:
                    line = messages.get(timeout=min(0.2, max(0.01, deadline - time.monotonic())))
                except queue.Empty:
                    self.assertIsNone(process.poll(), "Producer exited before readiness:\n" + "".join(output))
                    self.assertLess(time.monotonic(), deadline, "Readiness timeout:\n" + "".join(output))
                    continue
                if match := re.search(r"producer PID (\d+) -> viewer PID (\d+)", line):
                    self.assertEqual(int(match.group(1)), process.pid)
                    child = ChildObserver(int(match.group(2)))
                ready = ready or (line.strip() == "READY" if mode.startswith("dynamic_")
                                 else "Press Enter to exit..." in line)
            self.assertTrue(child.is_running())
            if mode in {"enter", "play"}:
                process.stdin.write("\n")
                process.stdin.flush()
            elif mode == "eof":
                process.stdin.close()
            elif mode in {"window", "dynamic_window", "dynamic_idle"}:
                child.close_window()
            elif mode in {"failure", "dynamic_failure"}:
                child.terminate()
            self.assertEqual(process.wait(timeout=10), 0, "".join(output))
            for reader in readers:
                reader.join(timeout=2)
            self.assertIn("DONE", "".join(output))
            self.assertFalse(child.is_running(), "Renderer child was orphaned")
            if mode == "interrupt":
                self.assertIn("INTERRUPTED", "".join(output))
            if mode == "failure":
                self.assertIn("CHILD_FAILED", "".join(output))
        finally:
            if child is not None:
                if child.is_running():
                    child.terminate()
                child.close()
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            for stream in (process.stdin, process.stdout, process.stderr):
                stream.close()

    def test_native_lifecycle(self):
        modes = ["enter", "eof", "play", "interrupt", "failure", "dynamic_failure"]
        if os.name == "nt":
            modes.extend(["window", "dynamic_window", "dynamic_idle"])
        for mode in modes:
            with self.subTest(mode=mode):
                self.exercise(mode)


if __name__ == "__main__":
    if "--child-test" in sys.argv:
        run_viewer(sys.argv[-1])
    elif "--gui" in sys.argv:
        sys.argv.remove("--gui")
        unittest.main()
    else:
        print("Skipped native GUI tests; pass --gui to open test windows.")
