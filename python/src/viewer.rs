use crate::notebook::setup_wasm_if_needed;
use crate::{Animation, Scene};
use cosmol_viewer_core::NativeGuiViewer;
use cosmol_viewer_wasm::NotebookViewer;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::OnceLock;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEnv {
    Colab,
    Jupyter,
    IPythonTerminal,
    IPythonOther,
    PlainScript,
    Unknown,
}

static RUNTIME_ENV: OnceLock<RuntimeEnv> = OnceLock::new();

impl std::fmt::Display for RuntimeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RuntimeEnv::Colab => "Colab",
            RuntimeEnv::Jupyter => "Jupyter",
            RuntimeEnv::IPythonTerminal => "IPython-Terminal",
            RuntimeEnv::IPythonOther => "Other IPython",
            RuntimeEnv::PlainScript => "Plain Script",
            RuntimeEnv::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

struct NotebookState {
    viewer: NotebookViewer,
    first_update: bool,
}

enum ViewerBackend {
    Colab(NotebookState),
    Jupyter(NotebookState),
    PlainScript(NativeGuiViewer),
    IPythonTerminal(NativeGuiViewer),
    Closed,
}

#[pyclass(module = "cosmol_viewer")]
#[pyo3(crate = "pyo3", unsendable)]
#[doc = r#"
A viewer for interactively displaying 3D scenes in different runtime
environments.

Depending on the runtime environment, the viewer automatically selects an
appropriate backend:

- Jupyter or Colab uses an inline WebAssembly canvas.
- A Python script or terminal uses a native GUI window when supported.

Use ``Viewer.render(scene, width, height)`` to display a scene interactively, or
``Viewer.play(animation, width, height)`` to play an animation. Use
``scene.save_image(path, width, height)`` or ``scene.to_png(width, height)`` for
static image output. Call ``viewer.keep_alive()`` at the end of a native script
to wait until Enter is pressed or the viewer window closes.
"#]
pub struct Viewer {
    backend: ViewerBackend,
}

fn detect_runtime_env(py: Python) -> PyResult<RuntimeEnv> {
    if let Some(env) = RUNTIME_ENV.get() {
        return Ok(*env);
    }

    let code = c_str!(
        r#"
def detect_env():
    import sys
    try:
        from IPython import get_ipython
        ipy = get_ipython()
        if ipy is None:
            return 'PlainScript'
        shell = ipy.__class__.__name__
        if 'google.colab' in sys.modules:
            return 'Colab'
        if shell == 'ZMQInteractiveShell':
            return 'Jupyter'
        elif shell == 'TerminalInteractiveShell':
            return 'IPython-Terminal'
        else:
            return f'IPython-{shell}'
    except:
        return 'PlainScript'
"#
    );

    let env_module = PyModule::from_code(py, code, c_str!("<detect_env>"), c_str!("env_module"))?;
    let fun = env_module.getattr("detect_env")?;
    let result: String = fun.call1(())?.extract()?;

    let env = match result.as_str() {
        "Colab" => RuntimeEnv::Colab,
        "Jupyter" => RuntimeEnv::Jupyter,
        "IPython-Terminal" => RuntimeEnv::IPythonTerminal,
        s if s.starts_with("IPython-") => RuntimeEnv::IPythonOther,
        "PlainScript" => RuntimeEnv::PlainScript,
        _ => RuntimeEnv::Unknown,
    };

    let _ = RUNTIME_ENV.set(env);
    Ok(env)
}

fn native_viewer_executable(py: Python) -> PyResult<PathBuf> {
    let executable = if let Some(path) = std::env::var_os("COSMOL_VIEWER_NATIVE_PATH") {
        PathBuf::from(path)
    } else {
        let native_package = py.import("cosmol_viewer_native").map_err(|_| {
            PyRuntimeError::new_err(
                "Native viewer package is missing. Reinstall cosmol-viewer, or build it with \
                 python python/build_native_viewer.py before maturin develop",
            )
        })?;
        let package_file: PathBuf = native_package.getattr("__file__")?.extract()?;
        package_file
            .parent()
            .ok_or_else(|| PyRuntimeError::new_err("Invalid native viewer package path"))?
            .join(format!(
                "cosmol-viewer-native{}",
                std::env::consts::EXE_SUFFIX
            ))
    };
    if !executable.is_file() {
        return Err(PyRuntimeError::new_err(format!(
            "Native viewer executable is missing: {}. Run python python/build_native_viewer.py \
             before maturin develop, or reinstall cosmol-viewer",
            executable.display()
        )));
    }
    Ok(executable)
}

#[pymethods]
impl Viewer {
    #[staticmethod]
    #[doc = r#"
Get the current runtime environment.

Returns
-------
str
    One of ``"Jupyter"``, ``"Colab"``, ``"Plain Script"``,
    ``"IPython-Terminal"``, ``"Other IPython"``, or ``"Unknown"``.
"#]
    pub fn get_environment(py: Python) -> PyResult<String> {
        let env = detect_runtime_env(py)?;
        Ok(env.to_string())
    }

    #[staticmethod]
    #[doc = r#"
Render a 3D scene.

Parameters
----------
scene : Scene
    The scene to render.
width : float
    The viewport width in pixels.
height : float
    The viewport height in pixels.

Returns
-------
Viewer
    The created viewer instance.
"#]
    pub fn render(scene: &Scene, width: f32, height: f32, py: Python) -> PyResult<Self> {
        let env_type = detect_runtime_env(py)?;
        match env_type {
            RuntimeEnv::Colab | RuntimeEnv::Jupyter => {
                setup_wasm_if_needed(py, env_type)?;
                let mut scene = scene.inner.clone();
                scene.prepare_for_wasm();
                let wasm_viewer = NotebookViewer::render(py, &scene, width, height)?;

                let state = NotebookState {
                    viewer: wasm_viewer,
                    first_update: true,
                };
                Ok(Viewer {
                    backend: if env_type == RuntimeEnv::Colab {
                        ViewerBackend::Colab(state)
                    } else {
                        ViewerBackend::Jupyter(state)
                    },
                })
            }
            RuntimeEnv::PlainScript | RuntimeEnv::IPythonTerminal => {
                let executable = native_viewer_executable(py)?;
                let native_gui_viewer = match NativeGuiViewer::render_with_executable(
                    &scene.inner,
                    width,
                    height,
                    executable,
                ) {
                    Ok(viewer) => viewer,
                    Err(err) => {
                        return Err(PyRuntimeError::new_err(format!(
                            "Error: Failed to initialize native GUI viewer: {:?}",
                            err
                        )));
                    }
                };

                Ok(Viewer {
                    backend: if env_type == RuntimeEnv::PlainScript {
                        ViewerBackend::PlainScript(native_gui_viewer)
                    } else {
                        ViewerBackend::IPythonTerminal(native_gui_viewer)
                    },
                })
            }
            _ => Err(PyValueError::new_err("Error: Invalid runtime environment")),
        }
    }

    #[staticmethod]
    #[doc = r#"
Play an animation.

Parameters
----------
animation : Animation
    An animation object containing frames and playback settings.
width : float
    The viewport width in pixels.
height : float
    The viewport height in pixels.

Returns
-------
Viewer
    The created viewer instance playing the animation.
"#]
    pub fn play(animation: Animation, width: f32, height: f32, py: Python) -> PyResult<Self> {
        if animation.inner.frames.is_empty() {
            return Err(PyErr::new::<PyRuntimeError, _>("No frames provided"));
        }
        let env_type = detect_runtime_env(py)?;

        match env_type {
            RuntimeEnv::Colab | RuntimeEnv::Jupyter => {
                setup_wasm_if_needed(py, env_type)?;
                let mut animation = animation.inner;
                if let Some(static_scene) = &mut animation.static_scene {
                    static_scene.prepare_for_wasm();
                }
                for frame in &mut animation.frames {
                    frame.prepare_for_wasm();
                }
                let wasm_viewer =
                    NotebookViewer::initiate_viewer_and_play(py, animation, width, height)?;

                let state = NotebookState {
                    viewer: wasm_viewer,
                    first_update: false,
                };
                Ok(Viewer {
                    backend: if env_type == RuntimeEnv::Colab {
                        ViewerBackend::Colab(state)
                    } else {
                        ViewerBackend::Jupyter(state)
                    },
                })
            }

            RuntimeEnv::PlainScript | RuntimeEnv::IPythonTerminal => {
                let executable = native_viewer_executable(py)?;
                let native_gui_viewer = NativeGuiViewer::play_with_executable(
                    animation.inner,
                    width,
                    height,
                    executable,
                )
                .map_err(|err| {
                    PyRuntimeError::new_err(format!(
                        "Error: Failed to initialize native GUI viewer: {err}"
                    ))
                })?;

                Ok(Viewer {
                    backend: if env_type == RuntimeEnv::PlainScript {
                        ViewerBackend::PlainScript(native_gui_viewer)
                    } else {
                        ViewerBackend::IPythonTerminal(native_gui_viewer)
                    },
                })
            }
            _ => Err(PyErr::new::<PyRuntimeError, _>(format!(
                "Invalid runtime environment {}",
                env_type
            ))),
        }
    }

    #[doc = r#"
Keep a native viewer alive until Enter is pressed or its child process exits.

This blocks the calling thread, but releases the Python GIL during short waits.
Enter or stdin EOF closes the child process. A normal child exit returns normally;
an unsuccessful child exit or an I/O failure raises ``OSError``. Ctrl+C raises
``KeyboardInterrupt`` and also closes the child process.

This is a finalization step: afterwards the viewer cannot be updated or reused,
including when an error interrupts the wait.

Raises
------
RuntimeError
    If called in Jupyter or Colab, or on an already finalized viewer.
OSError
    If console I/O fails or the child process exits unsuccessfully.
KeyboardInterrupt
    If the wait is interrupted by Ctrl+C.
"#]
    pub fn keep_alive(&mut self, py: Python) -> PyResult<()> {
        match &self.backend {
            ViewerBackend::Colab(_) | ViewerBackend::Jupyter(_) => {
                return Err(PyRuntimeError::new_err(
                    "keep_alive() is only available for native viewers, not Jupyter or Colab",
                ));
            }
            ViewerBackend::Closed => {
                return Err(PyRuntimeError::new_err("Viewer has already been finalized"));
            }
            _ => {}
        }

        let viewer = match std::mem::replace(&mut self.backend, ViewerBackend::Closed) {
            ViewerBackend::PlainScript(viewer) | ViewerBackend::IPythonTerminal(viewer) => viewer,
            _ => unreachable!(),
        };
        // Keep the non-Send IPC handles on their owning thread; only detach the sleep.
        viewer.keep_alive_with_wait(|duration| {
            py.detach(move || std::thread::sleep(duration));
            py.check_signals()
        })
    }

    #[doc = r#"
Check whether the native viewer is still open without blocking.

Use ``while viewer.is_open():`` for dynamic update loops. Returns ``False``
after the child process exits or the viewer is finalized. This does not read
console input; use ``keep_alive()`` for the final blocking wait with Enter.

Raises
------
RuntimeError
    If called in Jupyter or Colab, where child-process status is unavailable.
"#]
    pub fn is_open(&self) -> PyResult<bool> {
        match &self.backend {
            ViewerBackend::PlainScript(viewer) | ViewerBackend::IPythonTerminal(viewer) => {
                Ok(viewer.is_open())
            }
            ViewerBackend::Closed => Ok(false),
            ViewerBackend::Colab(_) | ViewerBackend::Jupyter(_) => Err(PyRuntimeError::new_err(
                "is_open() is only available for native viewers, not Jupyter or Colab",
            )),
        }
    }

    #[doc = r#"
Update the viewer with a new scene.

Parameters
----------
scene : Scene
    The updated scene.

Notes
-----
In Jupyter or Colab, frequent animation updates may be limited by notebook
rendering capacity, which can lead to delayed or incomplete rendering.
"#]
    pub fn update(&mut self, scene: &Scene, py: Python) -> PyResult<()> {
        match &mut self.backend {
            ViewerBackend::Colab(state) | ViewerBackend::Jupyter(state) => {
                if state.first_update {
                    print_to_notebook(
                        c_str!(
                            r###"print("\033[33m⚠️ Note: When running in Jupyter or Colab, animation updates may be limited by the notebook's output capacity, which can cause incomplete or delayed rendering.\033[0m")"###
                        ),
                        py,
                    );
                    state.first_update = false;
                }
                state.viewer.update(py, &scene.inner)?;
            }
            ViewerBackend::PlainScript(viewer) | ViewerBackend::IPythonTerminal(viewer) => {
                viewer.update(&scene.inner);
            }
            ViewerBackend::Closed => {
                return Err(PyRuntimeError::new_err("Viewer has already been finalized"));
            }
        }
        Ok(())
    }

    #[doc = r#"
Enable or disable logging camera parameters whenever the viewer camera moves.

When enabled, viewer interaction logs one line of orbit-style camera
parameters after drag, zoom, or auto-rotation changes the camera. The printed
values can be passed back to ``scene.set_camera_view(...)`` to reproduce the
view. Logging is disabled by default, and the angle conversion is only computed
while this option is enabled.

Native viewers log to the terminal. Notebook viewers log to the browser console;
the command is submitted without waiting for a browser response.

Parameters
----------
enabled : bool, optional
    Whether to print camera parameters on camera movement. Defaults to ``True``.
"#]
    #[pyo3(signature = (enabled=true))]
    pub fn camera_parameter_logging(&mut self, enabled: bool, py: Python) -> PyResult<()> {
        match &self.backend {
            ViewerBackend::PlainScript(viewer) | ViewerBackend::IPythonTerminal(viewer) => {
                viewer.camera_parameter_logging(enabled);
                Ok(())
            }
            ViewerBackend::Colab(state) | ViewerBackend::Jupyter(state) => {
                state.viewer.camera_parameter_logging(py, enabled)
            }
            ViewerBackend::Closed => {
                Err(PyRuntimeError::new_err("Viewer has already been finalized"))
            }
        }
    }

    #[doc = r#"
Show or hide the renderer FPS overlay in the upper-left corner.

The value is the repaint rate averaged over half-second windows, not the
animation frame rate or GPU timing. Samples existing repaint events only;
does not request continuous repainting. After one second without activity,
a one-shot repaint shows zero FPS; this diagnostic repaint is not counted
and does not renew the idle timer. Supported in native windows, Jupyter and Colab.

Parameters
----------
enabled : bool, optional
    Whether to show FPS. Defaults to ``True``; initially hidden.
"#]
    #[pyo3(signature = (enabled=true))]
    pub fn show_fps(&self, enabled: bool, py: Python) -> PyResult<()> {
        match &self.backend {
            ViewerBackend::PlainScript(viewer) | ViewerBackend::IPythonTerminal(viewer) => {
                viewer.show_fps(enabled);
                Ok(())
            }
            ViewerBackend::Colab(state) | ViewerBackend::Jupyter(state) => {
                state.viewer.show_fps(py, enabled)
            }
            ViewerBackend::Closed => {
                Err(PyRuntimeError::new_err("Viewer has already been finalized"))
            }
        }
    }

    #[doc = r#"
Show or hide live camera parameters in the upper-left corner.

Displays azimuth, elevation, roll (degrees), distance, target and field of view.
Updates with camera interaction and does not intercept dragging or zooming.
Independent of ``camera_parameter_logging()``; enabling this does not log
to the terminal or browser console. Supported in native windows, Jupyter and Colab.

Parameters
----------
enabled : bool, optional
    Whether to show camera parameters. Defaults to ``True``; initially hidden.
"#]
    #[pyo3(signature = (enabled=true))]
    pub fn show_camera_parameters(&self, enabled: bool, py: Python) -> PyResult<()> {
        match &self.backend {
            ViewerBackend::PlainScript(viewer) | ViewerBackend::IPythonTerminal(viewer) => {
                viewer.show_camera_parameters(enabled);
                Ok(())
            }
            ViewerBackend::Colab(state) | ViewerBackend::Jupyter(state) => {
                state.viewer.show_camera_parameters(py, enabled)
            }
            ViewerBackend::Closed => {
                Err(PyRuntimeError::new_err("Viewer has already been finalized"))
            }
        }
    }
}

fn print_to_notebook(msg: &CStr, py: Python) {
    let _ = py.run(msg, None, None);
}
