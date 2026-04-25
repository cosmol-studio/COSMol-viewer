use crate::shapes::py_to_color;
use cosmol_viewer_core::BUILD_ID;
use cosmol_viewer_core::scene::Animation as _Animation;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use std::env;
use std::ffi::CStr;
use std::sync::OnceLock;

use pyo3::{ffi::c_str, prelude::*};

use crate::shapes::{PyMolecule, PyProtein, PySphere, PyStick};
use cosmol_viewer_core::{NativeGuiViewer, scene::Scene as _Scene};
use cosmol_viewer_wasm::NotebookViewer;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

mod shapes;
mod utils;

#[derive(Clone)]
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[doc = r#"
A container for handling frame-based animations in the viewer.

Parameters
----------
interval : float
    Time in seconds between frames.
loops : int
    Number of times to loop the animation. Use ``-1`` for infinite looping.
interpolate : bool
    Whether to interpolate between frames for smoother visualization.


Examples
--------
.. code-block:: python

    animation = Animation(interval=0.1, loops=-1, interpolate=True)
    for _ in range(100):
        scene = Scene()
        scene.add_shape(..)
        animation.add_frame(scene)

    Viewer.play(animation, width=800.0, height=500.0)

"#]
pub struct Animation {
    inner: _Animation,
}

#[gen_stub_pymethods]
#[pymethods]
impl Animation {
    #[new]
    pub fn new(interval: f32, loops: i64, interpolate: bool) -> Self {
        Self {
            inner: _Animation {
                static_scene: None,
                frames: Vec::new(),
                interval: (interval * 1000.0) as u64,
                loops,
                interpolate,
            },
        }
    }

    #[doc = r#"
Add a frame to the animation.

Parameters
----------
scene : Scene
    A scene object representing a single frame of the animation.
"#]
    pub fn add_frame(&mut self, scene: Scene) {
        self.inner.frames.push(scene.inner);
    }

    #[doc = r#"
Set a static scene that remains constant throughout the animation.

Parameters
----------
scene : Scene
    A scene object to be rendered statically. This is useful for
    background elements or reference structures.
"#]
    pub fn set_static_scene(&mut self, scene: Scene) {
        self.inner.static_scene = Some(scene.inner);
    }

    #[gen_stub(skip)]
    fn __len__(&self) -> usize {
        self.inner.frames.len()
    }

    #[gen_stub(skip)]
    fn __repr__(&self) -> String {
        let interval_sec = self.inner.interval as f32 / 1000.0;
        let frames = self.inner.frames.len();

        format!(
            "Animation(frames={}, interval={:.3}s, loops={}, interpolate={})",
            frames, interval_sec, self.inner.loops, self.inner.interpolate
        )
    }

    #[gen_stub(skip)]
    fn __getitem__(&self, index: isize, py: Python) -> PyResult<Py<Scene>> {
        let frames = &self.inner.frames;

        let idx = if index >= 0 {
            index as usize
        } else {
            let abs = (-index) as usize;
            if abs > frames.len() {
                return Err(PyIndexError::new_err("Animation frame index out of range"));
            }
            frames.len() - abs
        };

        if idx >= frames.len() {
            return Err(PyIndexError::new_err("Animation frame index out of range"));
        }

        let scene_inner = frames[idx].clone();
        let py_scene = Scene { inner: scene_inner };

        Ok(Py::new(py, py_scene)?)
    }
}

#[derive(Clone)]
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[doc = r#"
A 3D scene container for visualizing molecular or geometric shapes.

This class allows adding, updating, and removing shapes in a scene,
as well as modifying scene-level properties such as scale and background color.

Supported shape types include ``Sphere``, ``Stick``, ``Molecule``, and ``Protein``.
Shapes can optionally be assigned a string ``id`` so they can be updated or removed later.

Examples
--------
.. code-block:: python

    scene = Scene()
"#]
pub struct Scene {
    inner: _Scene,
}

#[gen_stub_pymethods]
#[pymethods]
impl Scene {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: _Scene::new(),
        }
    }

    #[doc = r#"
Add a shape to the scene without assigning an explicit ID.

Parameters
----------
shape : Sphere or Stick or Molecule or Protein
    A shape instance to add to the scene.
"#]
    pub fn add_shape(&mut self, shape: &Bound<'_, PyAny>) -> PyResult<()> {
        macro_rules! try_add {
            ($py_type:ty) => {{
                if let Ok(py_obj) = shape.cast::<$py_type>() {
                    let py_obj = py_obj.borrow();
                    self.inner.add_shape(py_obj.inner.clone());
                    return Ok(());
                }
            }};
        }

        try_add!(PySphere);
        try_add!(PyStick);
        try_add!(PyMolecule);
        try_add!(PyProtein);

        let type_name = shape
            .get_type()
            .name()
            .map(|name| name.to_string())
            .unwrap_or("<unknown type>".to_string());

        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "add_shape(): unsupported shape type '{type_name}'. \
             Expected one of: Sphere, Stick, Molecule, Protein"
        )))
    }

    #[doc = r#"
Add a shape to the scene with a specific ID.

Parameters
----------
id : str
    A unique string identifier for the shape.
shape : Sphere or Stick or Molecule or Protein
    A shape instance to add to the scene.

Notes
-----
If a shape with the same ID already exists, this method may fail or behave strictly.
"#]
    pub fn add_shape_with_id(&mut self, id: &str, shape: &Bound<'_, PyAny>) -> PyResult<()> {
        macro_rules! try_add {
            ($py_type:ty) => {{
                if let Ok(py_obj) = shape.cast::<$py_type>() {
                    let py_obj = py_obj.borrow();
                    self.inner.add_shape_with_id(id, py_obj.inner.clone());
                    return Ok(());
                }
            }};
        }

        try_add!(PySphere);
        try_add!(PyStick);
        try_add!(PyMolecule);
        try_add!(PyProtein);

        let type_name = shape
            .get_type()
            .name()
            .map(|name| name.to_string())
            .unwrap_or("<unknown type>".to_string());

        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "add_shape_with_id(): unsupported shape type '{type_name}'. \
             Expected one of: Sphere, Stick, Molecule, Protein"
        )))
    }

    #[doc = r#"
Replace an existing shape in the scene by its ID.

Parameters
----------
id : str
    The ID of the shape to replace.
shape : Sphere or Stick or Molecule or Protein
    The new shape object.
"#]
    pub fn replace_shape(&mut self, id: &str, shape: &Bound<'_, PyAny>) -> PyResult<()> {
        macro_rules! update_with {
            ($py_type:ty) => {{
                if let Ok(py_obj) = shape.cast::<$py_type>() {
                    let py_obj = py_obj.borrow();
                    return self
                        .inner
                        .replace_shape(id, py_obj.inner.clone())
                        .map_err(|e| {
                            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
                        });
                }
            }};
        }

        update_with!(PySphere);
        update_with!(PyStick);
        update_with!(PyMolecule);
        update_with!(PyProtein);

        let type_name = shape
            .get_type()
            .name()
            .map(|name| name.to_string())
            .unwrap_or("<unknown type>".to_string());

        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "replace_shape(): unsupported type {type_name}",
        )))
    }

    #[doc = r#"
Remove a shape from the scene by its ID.

Parameters
----------
id : str
    The ID of the shape to remove.
"#]
    pub fn remove_shape(&mut self, id: &str) -> PyResult<()> {
        self.inner
            .remove_shape(id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[doc = r#"
Recenter the scene at a given point.

Parameters
----------
center : [float, float, float]
    The new scene center as XYZ coordinates.
"#]
    pub fn recenter(&mut self, center: [f32; 3]) {
        self.inner.recenter(center);
    }

    #[doc = r#"
Set the global scale factor of the scene.

Parameters
----------
scale : float
    A positive scaling factor applied uniformly to all shapes.
"#]
    pub fn set_scale(&mut self, scale: f32) {
        self.inner.set_scale(scale);
    }

    #[doc = r#"
Set the background color of the scene.

Parameters
----------
background_color : tuple[int, int, int] or str
    The background color as an RGB tuple or a hex string such as "\#FFFFFF".
"#]
    pub fn set_background_color(&mut self, background_color: Bound<'_, PyAny>) -> PyResult<()> {
        let color = py_to_color(background_color)?;
        self.inner.set_background_color(color);
        Ok(())
    }

    #[doc = r#"
Set the background color of the scene to black.
"#]
    pub fn use_black_background(&mut self) {
        self.inner.use_black_background();
    }

    #[gen_stub(skip)]
    fn __repr__(&self) -> String {
        format!("RustScene({:?})", self.inner)
    }
}

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

#[gen_stub_pyclass]
#[pyclass]
#[pyo3(crate = "pyo3", unsendable)]
#[doc = r#"
A viewer for rendering 3D scenes in different runtime environments.

Depending on the runtime environment, the viewer automatically selects an
appropriate backend:

- Jupyter or Colab uses an inline WebAssembly canvas.
- A Python script or terminal uses a native GUI window when supported.

Use ``Viewer.render(scene, width, height)`` to display a scene, or
``Viewer.play(animation, width, height)`` to play an animation.
"#]
pub struct Viewer {
    environment: RuntimeEnv,
    wasm_viewer: Option<NotebookViewer>,
    native_gui_viewer: Option<NativeGuiViewer>,
    first_update: bool,
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

#[gen_stub_pymethods]
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
                let wasm_viewer = NotebookViewer::initiate_viewer(py, &scene, width, height)?;

                Ok(Viewer {
                    environment: env_type,
                    wasm_viewer: Some(wasm_viewer),
                    native_gui_viewer: None,
                    first_update: true,
                })
            }
            RuntimeEnv::PlainScript | RuntimeEnv::IPythonTerminal => {
                let native_gui_viewer = match NativeGuiViewer::render(&scene.inner, width, height) {
                    Ok(viewer) => viewer,
                    Err(err) => {
                        return Err(PyRuntimeError::new_err(format!(
                            "Error: Failed to initialize native GUI viewer: {:?}",
                            err
                        )));
                    }
                };

                Ok(Viewer {
                    environment: env_type,
                    wasm_viewer: None,
                    native_gui_viewer: Some(native_gui_viewer),
                    first_update: true,
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
        let env_type = detect_runtime_env(py).unwrap();

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

                Ok(Viewer {
                    environment: env_type,
                    wasm_viewer: Some(wasm_viewer),
                    native_gui_viewer: None,
                    first_update: false,
                })
            }

            RuntimeEnv::PlainScript | RuntimeEnv::IPythonTerminal => {
                let _ = NativeGuiViewer::play(animation.inner, width, height);

                Ok(Viewer {
                    environment: env_type,
                    wasm_viewer: None,
                    native_gui_viewer: None,
                    first_update: false,
                })
            }
            _ => Err(PyErr::new::<PyRuntimeError, _>(format!(
                "Invalid runtime environment {}",
                env_type
            ))),
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
        let env_type = self.environment;
        match env_type {
            RuntimeEnv::Colab | RuntimeEnv::Jupyter => {
                if self.first_update {
                    print_to_notebook(
                        c_str!(
                            r###"print("\033[33m⚠️ Note: When running in Jupyter or Colab, animation updates may be limited by the notebook's output capacity, which can cause incomplete or delayed rendering.\033[0m")"###
                        ),
                        py,
                    );
                    self.first_update = false;
                }
                if let Some(ref wasm_viewer) = self.wasm_viewer {
                    wasm_viewer.update(py, &scene.inner)?;
                } else {
                    return Err(PyErr::new::<PyRuntimeError, _>(
                        "Viewer is not initialized properly",
                    ));
                }
            }
            RuntimeEnv::PlainScript | RuntimeEnv::IPythonTerminal => {
                if let Some(ref mut native_gui_viewer) = self.native_gui_viewer {
                    native_gui_viewer.update(&scene.inner);
                } else {
                    return Err(PyErr::new::<PyRuntimeError, _>(
                        "Viewer is not initialized properly",
                    ));
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    #[doc = r#"
Save the current image to a file.

This is supported by the native desktop backend and Colab. Jupyter image
saving is not fully supported yet.

Parameters
----------
path : str
    The file path where the image will be saved.
"#]
    pub fn save_image(&self, path: &str, py: Python) -> PyResult<()> {
        use std::fs;
        let env_type = self.environment;
        match env_type {
            RuntimeEnv::Colab => {
                if let Some(ref wasm_viewer) = self.wasm_viewer {
                    let img_buf_vec = wasm_viewer.take_screenshot_colab(py)?;
                    if let Err(e) = fs::write(path, &img_buf_vec) {
                        return Err(PyErr::new::<PyRuntimeError, _>(format!(
                            "Error saving image: {}",
                            e
                        )));
                    }
                } else {
                    return Err(PyErr::new::<PyRuntimeError, _>(
                        "Viewer is not initialized properly",
                    ));
                }
            }
            RuntimeEnv::Jupyter => {
                print_to_notebook(
                    c_str!(
                        r###"print("\033[33m⚠️ Image saving in Jupyter is not yet fully supported.\033[0m")"###
                    ),
                    py,
                );
            }
            RuntimeEnv::PlainScript | RuntimeEnv::IPythonTerminal => {
                let native_gui_viewer = &self.native_gui_viewer.as_ref().unwrap();
                let img = native_gui_viewer.take_screenshot();
                if let Err(e) = img.save(path) {
                    return Err(PyErr::new::<PyRuntimeError, _>(format!(
                        "Error saving image: {}",
                        e
                    )));
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

fn print_to_notebook(msg: &CStr, py: Python) {
    let _ = py.run(msg, None, None);
}

#[pymodule]
fn cosmol_viewer(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Scene>()?;
    m.add_class::<Animation>()?;
    m.add_class::<Viewer>()?;
    m.add_class::<PySphere>()?;
    m.add_class::<PyStick>()?;
    m.add_class::<PyMolecule>()?;
    m.add_class::<PyProtein>()?;
    Ok(())
}

pub fn setup_wasm_if_needed(py: Python, env: RuntimeEnv) -> PyResult<()> {
    use base64::Engine;
    use pyo3::types::PyAnyMethods;

    match env {
        RuntimeEnv::Colab => {}
        _ => (),
    }

    const JS_CODE: &str = include_str!("../../wasm/pkg/cosmol_viewer_wasm.js");
    const WASM_BYTES: &[u8] = include_bytes!("../../wasm/pkg/cosmol_viewer_wasm_bg.wasm");

    let js_base64 = base64::engine::general_purpose::STANDARD.encode(JS_CODE);
    let wasm_base64 = base64::engine::general_purpose::STANDARD.encode(WASM_BYTES);

    let combined_js = format!(
        r#"
(function() {{
    const version = "{BUILD_ID}";
    const ns = "cosmol_viewer_" + version;

    if (!window[ns + "_ready"]) {{
        // 1. setup JS module
        const jsCode = atob("{js_base64}");
        const jsBlob = new Blob([jsCode], {{ type: 'application/javascript' }});
        window[ns + "_blob_url"] = URL.createObjectURL(jsBlob);

        // 2. preload WASM
        const wasmBytes = Uint8Array.from(atob("{wasm_base64}"), c => c.charCodeAt(0));
        window[ns + "_wasm_bytes"] = wasmBytes;

        window[ns + "_ready"] = true;
        console.log("Cosmol viewer setup done, BUILD_ID:", version);
    }} else {{
        console.log("Cosmol viewer already set up, BUILD_ID:", version);
    }}
}})();
        "#,
        BUILD_ID = BUILD_ID,
        js_base64 = js_base64,
        wasm_base64 = wasm_base64
    );

    let ipython = py.import("IPython.display")?;
    let display = ipython.getattr("display")?;

    let js = ipython.getattr("Javascript")?.call1((combined_js,))?;
    display.call1((js,))?;

    Ok(())
}

use pyo3_stub_gen::define_stub_info_gatherer;
define_stub_info_gatherer!(stub_info);
