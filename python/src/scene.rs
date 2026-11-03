use crate::input::ColorInput;
use crate::selection::NativeSource;
use cosmol_viewer_core::scene::{Scene as _Scene, default_light_color};
use pyo3::prelude::*;

#[derive(Clone)]
#[pyclass(module = "cosmol_viewer", from_py_object)]
#[doc = r#"
A 3D scene container for visualizing molecular or geometric shapes.

This class allows adding, updating, and removing shapes in a scene,
as well as modifying scene-level properties such as scale, camera, and
background color. Static image export is also a scene-level operation via
``scene.save_image(...)`` and ``scene.to_png(...)``; it does not depend on a
``Viewer`` instance or the current notebook/browser/window size.

Positioned shapes include ``Sphere`` and ``Stick``. Use ``add`` or ``set`` with CK
objects, selections and styles for molecular/protein displays.
Shapes can optionally be assigned a string ``id`` so they can be updated or removed later.

Examples
--------
.. code-block:: python

    scene = Scene()
"#]
pub struct Scene {
    pub(crate) inner: _Scene,
    pub(crate) sources: crate::selection::Sources,
}

#[pymethods]
impl Scene {
    /// Add a positioned shape, CK object or source-bound selection.
    /// Omitted styles use the core defaults: molecules use BallAndStick, proteins use Cartoon.
    /// Selections use BallAndStick (molecule atoms), Stick (bonds), Sphere (protein atoms),
    /// or Cartoon (residues). Shapes already carry their own style.
    /// The native source import is shared by selections and Scenes.
    #[pyo3(signature = (item, style=None))]
    pub(crate) fn add(
        &mut self,
        item: crate::selection::SceneInput<'_>,
        style: Option<crate::styles::StyleInput>,
    ) -> PyResult<()> {
        let source = item.with_native(|item| {
            match style {
                Some(style) => self.inner.add((item, style.representation())),
                None => self.inner.add(item),
            }
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        })?;
        if let Some(source) = source {
            self.sources.retain(source);
        }
        Ok(())
    }
    /// Create or replace named content; failed projections preserve the old content.
    #[pyo3(signature = (name, item, style=None))]
    pub(crate) fn set(
        &mut self,
        name: &str,
        item: crate::selection::SceneInput<'_>,
        style: Option<crate::styles::StyleInput>,
    ) -> PyResult<()> {
        let source = item.with_native(|item| {
            match style {
                Some(style) => self.inner.set(name, (item, style.representation())),
                None => self.inner.set(name, item),
            }
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        })?;
        if let Some(source) = source {
            self.sources.retain(source);
        }
        Ok(())
    }
    pub(crate) fn recenter_on(
        &mut self,
        source: crate::selection::ScientificInput<'_>,
    ) -> PyResult<()> {
        let source = source.import()?;
        match &source.native {
            NativeSource::Molecule(molecule) => self.inner.recenter_on(molecule),
            NativeSource::Protein(protein) => self.inner.recenter_on(protein),
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        self.sources.retain(source);
        Ok(())
    }
    #[new]
    pub fn new() -> Self {
        Self {
            inner: _Scene::new(),
            sources: Default::default(),
        }
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
            .map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
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
    pub fn set_background_color(&mut self, background_color: ColorInput) -> PyResult<()> {
        let color = background_color.into_color()?;
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
    pub fn set_depth_cue_color(&mut self, color: ColorInput) -> PyResult<()> {
        let color = color.into_color()?;
        self.inner.set_depth_cue_color(color);
        Ok(())
    }

    #[doc = r##"
Set scene lighting parameters.

The defaults match the renderer's historical hard-coded lighting, so calling
this method with no arguments preserves the old appearance.

Parameters
----------
ambient : float, optional
    Global ambient light strength. Defaults to ``0.55``.
diffuse : float, optional
    Directional diffuse light strength. Defaults to ``0.65``.
specular : float, optional
    Multiplier for the existing material-dependent specular highlight curve.
    Defaults to ``1.0``.
intensity : float, optional
    Final directional light multiplier. Defaults to ``1.0``.
color : tuple[int, int, int] or str, optional
    RGB light color as a tuple or hex string such as ``"#fff7f7"``. Defaults
    to the previous light color ``[255, 247, 247]``.
"##]
    #[pyo3(signature = (ambient=0.55, diffuse=0.65, specular=1.0, intensity=1.0, color=None))]
    pub fn set_lighting(
        &mut self,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        intensity: f32,
        color: Option<ColorInput>,
    ) -> PyResult<()> {
        let color = match color {
            Some(color) => color.into_color()?.into(),
            None => default_light_color(),
        };
        self.inner
            .set_lighting(ambient, diffuse, specular, intensity, color);
        Ok(())
    }

    #[doc = r#"
Set the final directional light intensity multiplier.

Defaults to ``1.0``. Increasing this brightens the whole lit scene while keeping
the current ambient, diffuse, specular, and light color settings.
"#]
    pub fn set_light_intensity(&mut self, intensity: f32) {
        self.inner.set_light_intensity(intensity);
    }

    #[doc = r#"
Set the global ambient light strength.

Defaults to ``0.55``, matching the renderer's previous hard-coded ambient term.
"#]
    pub fn set_ambient_light(&mut self, intensity: f32) {
        self.inner.set_ambient_light(intensity);
    }

    #[doc = r##"
Set the scene light color.

The color affects both ambient and directional light. The default light color is
``[255, 247, 247]``, matching the renderer's previous hard-coded color.

Parameters
----------
color : tuple[int, int, int] or str
    RGB light color tuple or hex string such as ``"#fff7f7"``.
"##]
    pub fn set_light_color(&mut self, color: ColorInput) -> PyResult<()> {
        let color = color.into_color()?;
        self.inner.set_light_color(color);
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

    fn __repr__(&self) -> String {
        format!("RustScene({:?})", self.inner)
    }
}
