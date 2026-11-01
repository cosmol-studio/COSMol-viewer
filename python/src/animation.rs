use crate::scene::Scene;
use cosmol_viewer_core::scene::Animation as _Animation;
use cosmol_viewer_wasm::utils::encode_payload;
use pyo3::exceptions::{PyIndexError, PyRuntimeError};
use pyo3::prelude::*;
#[derive(Clone)]
#[pyclass(module = "cosmol_viewer", from_py_object)]
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
    pub(crate) inner: _Animation,
}

#[pymethods]
impl Animation {
    #[new]
    pub fn new(interval: f32, loops: i64, interpolate: bool) -> Self {
        Self {
            inner: _Animation::new(interval, loops, interpolate),
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
        self.inner.add_frame(scene.inner);
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
        self.inner.set_static_scene(scene.inner);
    }

    #[doc = r#"
Set the time between animation frames.

Parameters
----------
interval : float
    Time in seconds between frames. This uses the same unit as the
    ``Animation`` constructor and takes effect when the animation is played.
"#]
    pub fn set_interval(&mut self, interval: f32) {
        self.inner.set_interval(interval);
    }

    #[doc = r#"
Set the number of animation loops.

Parameters
----------
loops : int
    Number of times to loop the animation. Use ``-1`` for infinite looping.
"#]
    pub fn set_loops(&mut self, loops: i64) {
        self.inner.set_loops(loops);
    }

    #[doc = r#"
Enable or disable interpolation between animation frames.

Parameters
----------
interpolate : bool
    Whether to interpolate between compatible frames for smoother visualization.
"#]
    pub fn set_interpolate(&mut self, interpolate: bool) {
        self.inner.set_interpolate(interpolate);
    }

    #[doc = r#"
Serialize the animation into a versioned payload for direct browser playback.

The returned UTF-8 string can be written to a ``.cmv`` file and passed to
``Viewer.playNotebook(canvas_id, payload)`` in the WebAssembly
frontend. Static scenes and frames are prepared for WebAssembly rendering in
the serialized copy; this method does not modify the animation object.

Returns
-------
str
    A CMV2 Base64-encoded postcard payload, optionally gzip-compressed.

Raises
------
RuntimeError
    If the animation has no frames or cannot be serialized.
"#]
    pub fn to_payload(&self) -> PyResult<String> {
        if self.inner.frames.is_empty() {
            return Err(PyRuntimeError::new_err(
                "Animation must contain at least one frame",
            ));
        }

        let mut animation = self.inner.clone();
        if let Some(static_scene) = &mut animation.static_scene {
            static_scene.prepare_for_wasm();
        }
        for frame in &mut animation.frames {
            frame.prepare_for_wasm();
        }

        encode_payload(&animation)
            .map_err(|err| PyRuntimeError::new_err(format!("Failed to serialize animation: {err}")))
    }

    fn __len__(&self) -> usize {
        self.inner.frames.len()
    }

    fn __repr__(&self) -> String {
        let interval_sec = self.inner.interval as f32 / 1000.0;
        let frames = self.inner.frames.len();

        format!(
            "Animation(frames={}, interval={:.3}s, loops={}, interpolate={})",
            frames, interval_sec, self.inner.loops, self.inner.interpolate
        )
    }

    fn __getitem__(&self, index: isize, py: Python) -> PyResult<Py<Scene>> {
        let frames = &self.inner.frames;

        let idx = if index >= 0 {
            index as usize
        } else {
            let abs = index.unsigned_abs();
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
