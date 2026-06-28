use crate::shapes::py_to_color;
use cosmol_viewer_core::BUILD_ID;
use cosmol_viewer_core::scene::Animation as _Animation;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyBytes;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use pyo3::{ffi::c_str, prelude::*};

use crate::shapes::{PyMolecule, PyProtein, PySphere, PyStick};
use cosmol_viewer_core::{ImageBackground, ImageRenderer, NativeGuiViewer, scene::Scene as _Scene};
use cosmol_viewer_wasm::NotebookViewer;
#[cfg(feature = "stubgen")]
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
#[cfg(not(feature = "stubgen"))]
use pyo3_stub_gen_derive::remove_gen_stub;

mod shapes;
mod utils;

#[derive(Clone)]
#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
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

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
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
#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
#[pyclass(from_py_object)]
#[doc = r#"
A 3D scene container for visualizing molecular or geometric shapes.

This class allows adding, updating, and removing shapes in a scene,
as well as modifying scene-level properties such as scale, camera, and
background color. Static image export is also a scene-level operation via
``scene.save_image(...)`` and ``scene.to_png(...)``; it does not depend on a
``Viewer`` instance or the current notebook/browser/window size.

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

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
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
If a shape with the same ID already exists, it is replaced.
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
Enable or disable transparent scene background rendering.

Parameters
----------
enabled : bool, optional
    If ``True``, render the scene background with alpha 0 so browser canvas
    content behind the viewer can show through. Defaults to ``True``.
"#]
    #[pyo3(signature = (enabled=true))]
    pub fn set_transparent_background(&mut self, enabled: bool) {
        self.inner.set_transparent_background(enabled);
    }

    #[doc = r#"
Enable or disable interactive zooming for the scene.

Parameters
----------
disabled : bool, optional
    If ``True``, mouse wheel and trackpad scroll zooming are ignored while
    drag rotation remains enabled. Defaults to ``True``.
"#]
    #[pyo3(signature = (disabled=true))]
    pub fn set_zoom_disabled(&mut self, disabled: bool) {
        self.inner.set_zoom_disabled(disabled);
    }

    #[doc = r#"
Enable or disable automatic camera rotation for the scene.

Parameters
----------
enabled : bool, optional
    If ``True``, the camera keeps orbiting around the current target while
    normal drag rotation remains enabled. Defaults to ``True``.
speed : float, optional
    Horizontal orbit speed in degrees per second. Defaults to ``20.0``.
"#]
    #[pyo3(signature = (enabled=true, speed=20.0))]
    pub fn set_auto_rotate(&mut self, enabled: bool, speed: f32) {
        self.inner.set_auto_rotate(enabled, speed);
    }

    #[doc = r#"
Enable or disable depth cueing for the scene.

Depth cueing uses the ChimeraX model: fragments are linearly mixed toward
the depth cue color according to camera-space depth. Depth cueing is disabled
by default. When enabled, the default cue color follows the scene background
color, so distant fragments fade out. It applies to molecules, ribbons, and
protein surfaces.
"#]
    #[pyo3(signature = (enabled=true))]
    pub fn set_depth_cue(&mut self, enabled: bool) {
        self.inner.set_depth_cue(enabled);
    }

    #[doc = r#"
Set the fractional depth cue range.

Parameters
----------
start : float
    Fraction of the current scene depth range where dimming starts. Defaults
    to ``0.5``.
end : float
    Fraction of the current scene depth range where dimming reaches the
    depth cue color. Defaults to ``1.0``.
"#]
    pub fn set_depth_cue_range(&mut self, start: f32, end: f32) {
        self.inner.set_depth_cue_range(start, end);
    }

    #[doc = r##"
Set the depth cue color.

Parameters
----------
color : tuple[int, int, int] or str
    RGB color tuple or hex string such as ``"#FFFFFF"``. If unset, the scene
    background color is used.
"##]
    pub fn set_depth_cue_color(&mut self, color: Bound<'_, PyAny>) -> PyResult<()> {
        let color = py_to_color(color)?;
        self.inner.set_depth_cue_color(color);
        Ok(())
    }

    #[doc = r#"
Set the background color of the scene to black.
"#]
    pub fn use_black_background(&mut self) {
        self.inner.use_black_background();
    }

    #[doc = r#"
Set the camera with orbit-style angles.

Parameters
----------
azimuth : float, optional
    Horizontal camera angle in degrees around the target.
elevation : float, optional
    Vertical camera angle in degrees around the target.
roll : float, optional
    Camera roll in degrees.
distance : float, optional
    Distance from the camera to the target.
target : [float, float, float], optional
    XYZ point the camera looks at.
fov : float, optional
    Vertical field of view in degrees.
"#]
    #[pyo3(signature = (azimuth=0.0, elevation=0.0, roll=0.0, distance=35.0, target=[0.0, 0.0, 0.0], fov=15.0))]
    pub fn set_camera_view(
        &mut self,
        azimuth: f32,
        elevation: f32,
        roll: f32,
        distance: f32,
        target: [f32; 3],
        fov: f32,
    ) {
        self.inner
            .set_camera_view(azimuth, elevation, roll, distance, target, fov);
    }

    #[doc = r#"
Rotate the current camera by orbit-style angle deltas.

Parameters
----------
azimuth_delta : float
    Horizontal angle delta in degrees.
elevation_delta : float
    Vertical angle delta in degrees.
roll_delta : float, optional
    Roll angle delta in degrees.
"#]
    #[pyo3(signature = (azimuth_delta, elevation_delta, roll_delta=0.0))]
    pub fn rotate_camera(&mut self, azimuth_delta: f32, elevation_delta: f32, roll_delta: f32) {
        self.inner
            .rotate_camera(azimuth_delta, elevation_delta, roll_delta);
    }

    #[doc = r#"
Set the camera distance while keeping the current target and rotation.
"#]
    pub fn set_camera_distance(&mut self, distance: f32) {
        self.inner.set_camera_distance(distance);
    }

    #[doc = r#"
Set the camera target while keeping the current distance and rotation.
"#]
    pub fn set_camera_target(&mut self, target: [f32; 3]) {
        self.inner.set_camera_target(target);
    }

    #[doc = r#"
Set the camera field of view in degrees.
"#]
    pub fn set_camera_fov(&mut self, fov: f32) {
        self.inner.set_camera_fov(fov);
    }

    #[doc = r#"
Render this scene directly to a PNG file.

This path uses the native offscreen renderer and does not depend on a Viewer,
notebook JavaScript, or the current display size. On platforms with a headless
GL path, the in-process renderer avoids creating a GUI event loop. If the
in-process renderer cannot be created after a native viewer has already run,
``save_image`` automatically retries in an isolated Python subprocess. Set
``COSMOL_VIEWER_RENDER_ISOLATED=1`` to force the isolated path.

Parameters
----------
path : str
    Output PNG path.
width : int, optional
    Output image width in pixels.
height : int, optional
    Output image height in pixels.
background : str or [int, int, int] or [int, int, int, int], optional
    Export background. If omitted, use the scene background. Use
    ``"transparent"`` for a transparent PNG, a hex color such as ``#ffffff``,
    ``#ffffffff``, or an RGB/RGBA sequence.
"#]
    #[pyo3(signature = (path, width=800, height=600, background=None))]
    pub fn save_image(
        &self,
        path: &str,
        width: u32,
        height: u32,
        background: Option<Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let background = py_to_image_background(background)?;
        if render_images_in_child_process() {
            return render_scene_in_child_process(
                &self.inner,
                path,
                width,
                height,
                background,
                false,
            );
        }

        let output_path = absolutize_output_path(path)?;
        match ImageRenderer::save_png_with_background(
            &self.inner,
            &output_path,
            width,
            height,
            background.into(),
        ) {
            Ok(()) => Ok(()),
            Err(err) if should_retry_image_render_in_child_process(&err) => {
                render_scene_in_child_process(
                    &self.inner,
                    &path_to_string(&output_path),
                    width,
                    height,
                    background,
                    false,
                )
            }
            Err(err) => Err(PyRuntimeError::new_err(format!(
                "Error saving image: {err}"
            ))),
        }
    }

    #[doc = r#"
Render this scene directly to PNG bytes.

This is useful in notebooks when you want to display or store an image without
round-tripping through the browser canvas. On platforms with a headless GL path,
the in-process renderer avoids creating a GUI event loop. If the in-process
renderer cannot be created after a native viewer has already run, ``to_png``
automatically retries in an isolated Python subprocess. Set
``COSMOL_VIEWER_RENDER_ISOLATED=1`` to force the isolated path.

Parameters
----------
background : str or sequence, optional
    If omitted, use the scene background. Use ``"transparent"`` for transparent
    PNG output or pass a color such as ``#ffffff`` or ``[255, 255, 255, 255]``.
"#]
    #[pyo3(signature = (width=800, height=600, background=None))]
    pub fn to_png<'py>(
        &self,
        py: Python<'py>,
        width: u32,
        height: u32,
        background: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let background = py_to_image_background(background)?;
        if !render_images_in_child_process() {
            match ImageRenderer::render_png_bytes_with_background(
                &self.inner,
                width,
                height,
                background.into(),
            ) {
                Ok(bytes) => return Ok(PyBytes::new(py, &bytes)),
                Err(err) if !should_retry_image_render_in_child_process(&err) => {
                    return Err(PyRuntimeError::new_err(format!(
                        "Error rendering image: {err}"
                    )));
                }
                Err(_) => {}
            }
        }

        let bytes =
            render_scene_to_png_bytes_in_child_process(&self.inner, width, height, background)?;
        Ok(PyBytes::new(py, &bytes))
    }

    #[doc = r#"
Display this scene as a static PNG in a notebook.

This method does not create an interactive viewer. It renders through the same
offscreen path as ``scene.to_png(...)`` and displays the resulting PNG with
``IPython.display``. The optional ``background`` argument follows
``scene.to_png(...)``.
"#]
    #[pyo3(signature = (width=1200, height=900, background=None))]
    pub fn display(
        &self,
        py: Python,
        width: u32,
        height: u32,
        background: Option<Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let bytes = self.to_png(py, width, height, background)?;
        let display_mod = py.import("IPython.display").map_err(|err| {
            PyRuntimeError::new_err(format!(
                "scene.display(...) requires IPython.display: {err}"
            ))
        })?;
        let image = display_mod.getattr("Image")?.call1((bytes,))?;
        display_mod.call_method1("display", (image,))?;
        Ok(())
    }

    #[staticmethod]
    #[gen_stub(skip)]
    pub fn _render_json_to_png_file(
        json_path: &str,
        output_path: &str,
        width: u32,
        height: u32,
        background_json: &str,
    ) -> PyResult<()> {
        let json = std::fs::read_to_string(json_path)
            .map_err(|err| PyRuntimeError::new_err(format!("Error reading scene JSON: {err}")))?;
        let scene: _Scene = serde_json::from_str(&json)
            .map_err(|err| PyRuntimeError::new_err(format!("Error parsing scene JSON: {err}")))?;
        let background: StaticImageBackground = serde_json::from_str(background_json)
            .map_err(|err| PyRuntimeError::new_err(format!("Error parsing background: {err}")))?;
        ImageRenderer::save_png_with_background(
            &scene,
            output_path,
            width,
            height,
            background.into(),
        )
        .map_err(|err| PyRuntimeError::new_err(format!("Error saving image: {err}")))
    }

    #[gen_stub(skip)]
    fn __repr__(&self) -> String {
        format!("RustScene({:?})", self.inner)
    }
}

fn render_scene_in_child_process(
    scene: &_Scene,
    output_path: &str,
    width: u32,
    height: u32,
    background: StaticImageBackground,
    remove_output_on_error: bool,
) -> PyResult<()> {
    let scene_path = unique_temp_path("cosmol_viewer_scene", "json")?;
    let json = serde_json::to_string(scene)
        .map_err(|err| PyRuntimeError::new_err(format!("Error serializing scene: {err}")))?;
    std::fs::write(&scene_path, json)
        .map_err(|err| PyRuntimeError::new_err(format!("Error writing scene JSON: {err}")))?;

    let output_path = absolutize_output_path(output_path)?;
    let script = concat!(
        "import sys\n",
        "from cosmol_viewer import Scene\n",
        "Scene._render_json_to_png_file(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5])\n",
    );
    let background_json = serde_json::to_string(&background)
        .map_err(|err| PyRuntimeError::new_err(format!("Error serializing background: {err}")))?;

    let python_exe = std::env::current_exe().map_err(|err| {
        PyRuntimeError::new_err(format!("Error locating Python executable: {err}"))
    })?;
    let output = Command::new(python_exe)
        .arg("-c")
        .arg(script)
        .arg(path_to_string(&scene_path))
        .arg(path_to_string(&output_path))
        .arg(width.to_string())
        .arg(height.to_string())
        .arg(background_json)
        .output()
        .map_err(|err| PyRuntimeError::new_err(format!("Error launching image renderer: {err}")))?;

    let _ = std::fs::remove_file(scene_path);

    if output.status.success() {
        return Ok(());
    }

    if remove_output_on_error {
        let _ = std::fs::remove_file(&output_path);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(PyRuntimeError::new_err(format!(
        "Image renderer failed with status {}.\nstdout:\n{}\nstderr:\n{}",
        output.status, stdout, stderr
    )))
}

fn render_scene_to_png_bytes_in_child_process(
    scene: &_Scene,
    width: u32,
    height: u32,
    background: StaticImageBackground,
) -> PyResult<Vec<u8>> {
    let output_path = unique_temp_path("cosmol_viewer_image", "png")?;
    render_scene_in_child_process(
        scene,
        &path_to_string(&output_path),
        width,
        height,
        background,
        true,
    )?;
    let bytes = std::fs::read(&output_path)
        .map_err(|err| PyRuntimeError::new_err(format!("Error reading rendered image: {err}")))?;
    let _ = std::fs::remove_file(output_path);
    Ok(bytes)
}

fn render_images_in_child_process() -> bool {
    std::env::var("COSMOL_VIEWER_RENDER_ISOLATED")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn should_retry_image_render_in_child_process(error: &str) -> bool {
    error.contains("EventLoop can't be recreated")
        || error.contains("Offscreen rendering could not create its GL bootstrap event loop")
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum StaticImageBackground {
    Scene,
    Color([f32; 4]),
}

impl From<StaticImageBackground> for ImageBackground {
    fn from(background: StaticImageBackground) -> Self {
        match background {
            StaticImageBackground::Scene => ImageBackground::Scene,
            StaticImageBackground::Color(color) => ImageBackground::Color(color),
        }
    }
}

fn py_to_image_background(background: Option<Bound<'_, PyAny>>) -> PyResult<StaticImageBackground> {
    let Some(background) = background else {
        return Ok(StaticImageBackground::Scene);
    };

    if let Ok(s) = background.extract::<String>() {
        let normalized = s.trim().to_ascii_lowercase();
        if normalized == "scene" || normalized == "default" {
            return Ok(StaticImageBackground::Scene);
        }
        if normalized == "transparent" || normalized == "none" {
            return Ok(StaticImageBackground::Color([0.0, 0.0, 0.0, 0.0]));
        }
        if let Some(color) = parse_hex_background(&s)? {
            return Ok(StaticImageBackground::Color(color));
        }
    }

    if let Ok(v) = background.extract::<[i64; 4]>() {
        return Ok(StaticImageBackground::Color([
            py_color_channel_to_f32(v[0])?,
            py_color_channel_to_f32(v[1])?,
            py_color_channel_to_f32(v[2])?,
            py_color_channel_to_f32(v[3])?,
        ]));
    }

    if let Ok(v) = background.extract::<[f64; 4]>() {
        return Ok(StaticImageBackground::Color([
            py_float_channel_to_f32(v[0])?,
            py_float_channel_to_f32(v[1])?,
            py_float_channel_to_f32(v[2])?,
            py_float_channel_to_f32(v[3])?,
        ]));
    }

    let color = py_to_color(background)?;
    let color = color.0;
    Ok(StaticImageBackground::Color([
        color.x, color.y, color.z, 1.0,
    ]))
}

fn parse_hex_background(value: &str) -> PyResult<Option<[f32; 4]>> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 8 {
        return Ok(None);
    }

    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16).map_err(|err| {
            PyValueError::new_err(format!("Invalid hex background color '{value}': {err}"))
        })
    };
    let r = parse(0..2)? as f32 / 255.0;
    let g = parse(2..4)? as f32 / 255.0;
    let b = parse(4..6)? as f32 / 255.0;
    let a = if hex.len() == 8 {
        parse(6..8)? as f32 / 255.0
    } else {
        1.0
    };
    Ok(Some([r, g, b, a]))
}

fn py_color_channel_to_f32(value: i64) -> PyResult<f32> {
    if !(0..=255).contains(&value) {
        return Err(PyValueError::new_err(format!(
            "Background color channel {value} out of range [0, 255]"
        )));
    }
    Ok(value as f32 / 255.0)
}

fn py_float_channel_to_f32(value: f64) -> PyResult<f32> {
    if !(0.0..=1.0).contains(&value) {
        return Err(PyValueError::new_err(format!(
            "Background float channel {value} out of range [0.0, 1.0]"
        )));
    }
    Ok(value as f32)
}

fn unique_temp_path(prefix: &str, extension: &str) -> PyResult<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| PyRuntimeError::new_err(format!("System clock error: {err}")))?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "{}_{}_{}.{}",
        prefix,
        std::process::id(),
        nanos,
        extension
    )))
}

fn absolutize_output_path(path: &str) -> PyResult<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| PyRuntimeError::new_err(format!("Error resolving output path: {err}")))
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
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

#[cfg_attr(feature = "stubgen", gen_stub_pyclass)]
#[pyclass]
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
static image output.
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

#[cfg_attr(feature = "stubgen", gen_stub_pymethods)]
#[cfg_attr(not(feature = "stubgen"), remove_gen_stub)]
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

#[cfg(feature = "stubgen")]
use pyo3_stub_gen::define_stub_info_gatherer;
#[cfg(feature = "stubgen")]
define_stub_info_gatherer!(stub_info);
