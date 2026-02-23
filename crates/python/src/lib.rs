use cosmol_viewer_core::BUILD_ID;
use cosmol_viewer_core::scene::Animation as _Animation;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use std::env;
use std::ffi::CStr;

use pyo3::{ffi::c_str, prelude::*};

use crate::shapes::{PyMolecule, PyProtein, PySphere, PyStick};
use cosmol_viewer_core::{NativeGuiViewer, scene::Scene as _Scene};
use cosmol_viewer_wasm::NotebookViewer;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

mod shapes;

#[derive(Clone)]
#[gen_stub_pyclass]
#[pyclass(from_py_object)]
#[doc = r#"
    A container for handling frame-based animations in the viewer.
"#]
pub struct Animation {
    inner: _Animation,
}

#[gen_stub_pymethods]
#[pymethods]
impl Animation {
    #[new]
    #[doc = r#"
        Create a new Animation container.

        # Args
        - interval: Time in seconds between frames.
        - loops: Number of times to loop the animation (-1 for infinite).
        - smooth: Whether to interpolate between frames for smoother visualization.

        # Example
        ```python
        anim = Animation(interval=0.1, loops=-1, smooth=True)
        ```
    "#]
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
        Add a frame (Scene) to the animation.

        # Args
        - frame: A Scene object representing a single frame of the animation.
    "#]
    pub fn add_frame(&mut self, frame: Scene) {
        self.inner.frames.push(frame.inner);
    }

    #[doc = r#"
        Set a static scene that remains constant throughout the animation.
        Useful for background elements or reference structures.

        # Args
        - scene: A Scene object to be rendered statically.
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
            "Animation(frames={}, interval={:.3}s, loops={}, smooth={})",
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

    This class allows adding, updating, and removing shapes in a 3D scene,
    as well as modifying scene-level properties like scale and background color.

    Supported shape types:
    - Sphere
    - Stick
    - Molecule
    - Protein

    Shapes can be optionally identified with a string `id`,
    which allows updates and deletion.
"#]
pub struct Scene {
    inner: _Scene,
}

#[gen_stub_pymethods]
#[pymethods]
impl Scene {
    #[new]
    #[doc = r#"
        Creates a new empty scene.

        # Example
        ```python
        scene = Scene()
        ```
    "#]
    pub fn new() -> Self {
        Self {
            inner: _Scene::new(),
        }
    }

    #[doc = r#"
        Add a shape to the scene without an explicit ID.

        # Args
        - shape: A shape instance (Sphere, Stick, Molecule, or Protein).

        # Example
        ```python
        scene.add_shape(sphere)
        ```
    "#]
    pub fn add_shape(&mut self, shape: &Bound<'_, PyAny>) -> PyResult<()> {
        macro_rules! try_add {
            ($py_type:ty) => {{
                if let Ok(py_obj) = shape.extract::<PyRef<$py_type>>() {
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
        Add a shape to the scene with a specific ID which can be used to update or remove the shape later.
        If a shape with the same ID exists, this method may fail or behave strictly;

        # Args
        - id: Unique string ID for the shape.
        - shape: A shape instance.

        # Example
        ```python
        scene.add_shape_with_id("bond1", stick)
        ```
    "#]
    pub fn add_shape_with_id(&mut self, id: &str, shape: &Bound<'_, PyAny>) -> PyResult<()> {
        macro_rules! try_add {
            ($py_type:ty) => {{
                if let Ok(py_obj) = shape.extract::<PyRef<$py_type>>() {
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
            "add_shape(): unsupported shape type '{type_name}'. \
             Expected one of: Sphere, Stick, Molecules, Protein"
        )))
    }

    #[doc = r#"
        Replace an existing shape in the scene by its ID.

        # Args
        - id: ID of the shape to update.
        - shape: New shape object to replace the existing one.

        # Example
        ```python
        scene.replace_shape("mol", updated_molecule)
        ```
    "#]
    pub fn replace_shape(&mut self, id: &str, shape: &Bound<'_, PyAny>) -> PyResult<()> {
        macro_rules! update_with {
            ($py_type:ty) => {{
                if let Ok(py_obj) = shape.extract::<PyRef<$py_type>>() {
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

        # Args
        - id: ID of the shape to remove.

        # Example
        ```python
        scene.remove_shape("bond1")
        ```
    "#]
    pub fn remove_shape(&mut self, id: &str) -> PyResult<()> {
        self.inner
            .remove_shape(id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[doc = r#"
        Recenter the scene at a given point.

        # Args
        - center: An XYZ array of 3 float values representing the new center.

        # Example
        ```python
        scene.recenter([0.0, 0.0, 0.0])
        ```
    "#]
    pub fn recenter(&mut self, center: [f32; 3]) {
        self.inner.recenter(center);
    }

    #[doc = r#"
        Set the global scale factor of the scene.
        This affects the visual size of all shapes uniformly.

        # Args
        - scale: A positive float scaling factor.

        # Example
        ```python
        scene.set_scale(1.5)
        ```
    "#]
    pub fn set_scale(&mut self, scale: f32) {
        self.inner.set_scale(scale);
    }

    #[doc = r#"
        Set the background color of the scene.

        # Args
        - background_color: An RGB array of 3 float values between 0.0 and 1.0.

        # Example
        ```python
        scene.set_background_color([1.0, 1.0, 1.0]) # white background
        ```
    "#]
    pub fn set_background_color(&mut self, background_color: [f32; 3]) {
        self.inner.set_background_color(background_color);
    }

    #[doc = r#"
        Set the background color of the scene to black.

        # Example
        ```python
        scene.use_black_background()
        ```
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
    A viewer that renders 3D scenes in different runtime environments
    (e.g., Jupyter, Colab, or native GUI).

    The `Viewer` automatically selects a backend:
    - Jupyter/Colab → WebAssembly canvas (inline display)
    - Python script/terminal → native GUI window (if supported)

    Use `Viewer.render(scene)` to create and display a viewer instance.
"#]
pub struct Viewer {
    environment: RuntimeEnv,
    wasm_viewer: Option<NotebookViewer>,
    native_gui_viewer: Option<NativeGuiViewer>,
    first_update: bool,
}

fn detect_runtime_env(py: Python) -> PyResult<RuntimeEnv> {
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

    Ok(env)
}

#[gen_stub_pymethods]
#[pymethods]
impl Viewer {
    #[staticmethod]
    #[doc = r#"
        Get the current runtime environment.

        # Returns
        - str: One of "Jupyter", "Colab", "PlainScript", or "IPythonTerminal".

        # Example
        ```python
        env = Viewer.get_environment()
        print(env) # e.g., "Jupyter"
        ```
    "#]
    pub fn get_environment(py: Python) -> PyResult<String> {
        let env = detect_runtime_env(py)?;
        Ok(env.to_string())
    }

    #[staticmethod]
    #[doc = r#"
        Render a 3D scene.

        # Args
        - scene: The scene to render.
        - width: The viewport width in pixels (default: 800).
        - height: The viewport height in pixels (default: 600).

        # Returns
        - Viewer: The created viewer instance.

        # Example
        ```python
        from cosmol_viewer import Viewer, Scene, Sphere
        scene = Scene()
        scene.add_shape(Sphere([0, 0, 0], 1.0))
        viewer = Viewer.render(scene)
        ```
    "#]
    pub fn render(scene: &Scene, width: f32, height: f32, py: Python) -> PyResult<Self> {
        let env_type = detect_runtime_env(py)?;
        match env_type {
            RuntimeEnv::Colab | RuntimeEnv::Jupyter => {
                setup_wasm_if_needed(py, env_type)?;
                let wasm_viewer = NotebookViewer::initiate_viewer(py, &scene.inner, width, height)?;

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

        # Args
        - animation: An Animation object containing frames and settings.
        - width: The viewport width in pixels.
        - height: The viewport height in pixels.

        # Returns
        - Viewer: The created viewer instance playing the animation.

        # Example
        ```python
        from cosmol_viewer import Viewer, Animation
        anim = Animation(0.1, 10, True)
        # ... add frames to anim ...
        viewer = Viewer.play(anim, 800.0, 600.0)
        ```
    "#]
    pub fn play(animation: Animation, width: f32, height: f32, py: Python) -> PyResult<Self> {
        if animation.inner.frames.is_empty() {
            return Err(PyErr::new::<PyRuntimeError, _>("No frames provided"));
        }
        let env_type = detect_runtime_env(py).unwrap();

        match env_type {
            RuntimeEnv::Colab | RuntimeEnv::Jupyter => {
                setup_wasm_if_needed(py, env_type)?;
                let wasm_viewer =
                    NotebookViewer::initiate_viewer_and_play(py, animation.inner, width, height)?;

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
        Works for both Web-based rendering (Jupyter/Colab) and native GUI windows.

        ⚠️ Note (Jupyter/Colab): Animation updates may be limited by notebook rendering capacity.

        # Args
        - scene: The updated scene.

        # Example
        ```python
        scene.add_shape(Sphere([1, 1, 1], 0.5))
        viewer.update(scene)
        ```
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

        # Args
        - path: File path for the saved image.

        # Example
        ```python
        viewer.save_image("output.png")
        ```
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
                // test code
                // if let Some(ref wasm_viewer) = self.wasm_viewer {
                //     let img_buf_vec = wasm_viewer.take_screenshot_jupyter(py)?;
                //     if let Err(e) = fs::write(path, &img_buf_vec) {
                //         return Err(PyErr::new::<PyRuntimeError, _>(format!(
                //             "Error saving image: {}",
                //             e
                //         )));
                //     }
                // } else {
                //     return Err(PyErr::new::<PyRuntimeError, _>(
                //         "Viewer is not initialized properly",
                //     ));
                // }
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
