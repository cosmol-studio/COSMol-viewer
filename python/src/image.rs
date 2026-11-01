//! Offscreen image projection and isolated renderer lifecycle.
use crate::input::ColorInput;
use crate::scene::Scene;
use cosmol_viewer_core::{ImageBackground, ImageRenderer, scene::Scene as _Scene};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[pymethods]
impl Scene {
    #[doc = r#"
Render this scene directly to a PNG file.

This path uses the native offscreen renderer and does not depend on a Viewer,
notebook JavaScript, or the current display size. On platforms with a headless
GL path, the in-process renderer avoids creating a GUI event loop. If the
in-process renderer cannot be created after a native viewer has already run,
``save_image`` automatically retries in an isolated Python subprocess. Google
Colab uses the isolated software-rendering path automatically. Set
``COSMOL_VIEWER_RENDER_ISOLATED=1`` to force the isolated path elsewhere.

Parameters
----------
path : str or os.PathLike[str]
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
        path: PathBuf,
        width: u32,
        height: u32,
        background: Option<BackgroundInput>,
    ) -> PyResult<()> {
        let background = image_background(background)?;
        if render_images_in_child_process() {
            return render_scene_in_child_process(
                &self.inner,
                &path,
                width,
                height,
                background,
                false,
            );
        }

        let output_path = absolutize_output_path(&path)?;
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
                    &output_path,
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
automatically retries in an isolated Python subprocess. Google Colab uses the
isolated software-rendering path automatically. Set
``COSMOL_VIEWER_RENDER_ISOLATED=1`` to force the isolated path elsewhere.

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
        background: Option<BackgroundInput>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let background = image_background(background)?;
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
        background: Option<BackgroundInput>,
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

    pub fn _render_json_to_png_file(
        json_path: &str,
        output_path: PathBuf,
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
}

fn render_scene_in_child_process(
    scene: &_Scene,
    output_path: &Path,
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
        "import os\n",
        "import sys\n",
        "from cosmol_viewer import Scene\n",
        "Scene._render_json_to_png_file(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5])\n",
        // The process exists solely to isolate native rendering. Once the PNG
        // is complete, skip Python extension and Mesa process-exit teardown.
        "os._exit(0)\n",
    );
    let background_json = serde_json::to_string(&background)
        .map_err(|err| PyRuntimeError::new_err(format!("Error serializing background: {err}")))?;

    let python_exe = std::env::current_exe().map_err(|err| {
        PyRuntimeError::new_err(format!("Error locating Python executable: {err}"))
    })?;
    let arguments = ChildRenderArguments {
        script,
        scene_path: &scene_path,
        output_path: &output_path,
        width,
        height,
        background_json: &background_json,
    };
    let colab = is_google_colab_environment();
    let output = run_image_renderer(&python_exe, &arguments, colab)?;

    let _ = std::fs::remove_file(scene_path);

    if output.status.success() {
        return Ok(());
    }

    if remove_output_on_error {
        let _ = std::fs::remove_file(&output_path);
    }

    Err(PyRuntimeError::new_err(format!(
        "Image renderer failed.\n{}",
        format_child_failure("image renderer", &output)
    )))
}

struct ChildRenderArguments<'a> {
    script: &'a str,
    scene_path: &'a Path,
    output_path: &'a Path,
    width: u32,
    height: u32,
    background_json: &'a str,
}

fn run_image_renderer(
    python_exe: &Path,
    arguments: &ChildRenderArguments<'_>,
    colab: bool,
) -> PyResult<Output> {
    let mut command = Command::new(python_exe);
    command
        .arg("-c")
        .arg(arguments.script)
        .arg(arguments.scene_path)
        .arg(arguments.output_path)
        .arg(arguments.width.to_string())
        .arg(arguments.height.to_string())
        .arg(arguments.background_json);

    configure_colab_render_environment(&mut command, colab, |name| std::env::var_os(name));

    command
        .output()
        .map_err(|err| PyRuntimeError::new_err(format!("Error launching image renderer: {err}")))
}

fn configure_colab_render_environment(
    command: &mut Command,
    colab: bool,
    mut get_var: impl FnMut(&str) -> Option<OsString>,
) {
    if !colab {
        return;
    }

    if get_var("LIBGL_ALWAYS_SOFTWARE").is_none() {
        command.env("LIBGL_ALWAYS_SOFTWARE", "1");
    }
    // Colab's llvmpipe JIT can crash inside Mesa even with a single-sample
    // framebuffer. Softpipe is slower but stays isolated and deterministic.
    if get_var("GALLIUM_DRIVER").is_none() {
        command.env("GALLIUM_DRIVER", "softpipe");
    }
    command.env("COSMOL_VIEWER_OFFSCREEN_TRACE", "1");
}

fn format_child_failure(label: &str, output: &Output) -> String {
    format!(
        "{label} failed with status {}.\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn render_scene_to_png_bytes_in_child_process(
    scene: &_Scene,
    width: u32,
    height: u32,
    background: StaticImageBackground,
) -> PyResult<Vec<u8>> {
    let output_path = unique_temp_path("cosmol_viewer_image", "png")?;
    render_scene_in_child_process(scene, &output_path, width, height, background, true)?;
    let bytes = std::fs::read(&output_path)
        .map_err(|err| PyRuntimeError::new_err(format!("Error reading rendered image: {err}")))?;
    let _ = std::fs::remove_file(output_path);
    Ok(bytes)
}

fn render_images_in_child_process() -> bool {
    std::env::var("COSMOL_VIEWER_RENDER_ISOLATED")
        .map(|value| environment_flag_enabled(&value))
        .unwrap_or_else(|_| is_google_colab_environment())
}

fn environment_flag_enabled(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn is_google_colab_environment() -> bool {
    is_google_colab_environment_with(|name| std::env::var_os(name))
}

fn is_google_colab_environment_with(mut get_var: impl FnMut(&str) -> Option<OsString>) -> bool {
    const COLAB_MARKERS: [&str; 4] = [
        "COLAB_RELEASE_TAG",
        "COLAB_BACKEND_VERSION",
        "COLAB_JUPYTER_TRANSPORT",
        "COLAB_GPU",
    ];

    COLAB_MARKERS.iter().any(|name| get_var(name).is_some())
}

fn should_retry_image_render_in_child_process(error: &str) -> bool {
    error.contains("EventLoop can't be recreated")
        || error.contains("Offscreen rendering could not create its GL bootstrap event loop")
}

#[cfg(test)]
mod environment_tests {
    use super::{
        configure_colab_render_environment, environment_flag_enabled,
        is_google_colab_environment_with,
    };
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::process::Command;

    #[test]
    fn parses_enabled_environment_flags() {
        for value in ["1", "true", "TRUE", "yes", "ON"] {
            assert!(environment_flag_enabled(value));
        }
        for value in ["0", "false", "no", "off", "invalid"] {
            assert!(!environment_flag_enabled(value));
        }
    }

    #[test]
    fn detects_google_colab_from_runtime_markers() {
        for marker in [
            "COLAB_RELEASE_TAG",
            "COLAB_BACKEND_VERSION",
            "COLAB_JUPYTER_TRANSPORT",
            "COLAB_GPU",
        ] {
            let environment = HashMap::from([(marker, OsString::from("present"))]);
            assert!(is_google_colab_environment_with(|name| environment
                .get(name)
                .cloned()));
        }
    }

    #[test]
    fn does_not_treat_a_regular_notebook_as_google_colab() {
        let environment = HashMap::from([("JPY_PARENT_PID", OsString::from("123"))]);
        assert!(!is_google_colab_environment_with(|name| environment
            .get(name)
            .cloned()));
    }

    #[test]
    fn configures_safe_colab_software_rendering_defaults() {
        let mut command = Command::new("python");
        configure_colab_render_environment(&mut command, true, |_| None);

        let configured = command
            .get_envs()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(
            configured.get("LIBGL_ALWAYS_SOFTWARE"),
            Some(&Some("1".to_owned()))
        );
        assert_eq!(
            configured.get("GALLIUM_DRIVER"),
            Some(&Some("softpipe".to_owned()))
        );
        assert_eq!(
            configured.get("COSMOL_VIEWER_OFFSCREEN_TRACE"),
            Some(&Some("1".to_owned()))
        );
    }

    #[test]
    fn preserves_explicit_colab_renderer_selection() {
        let environment = HashMap::from([
            ("LIBGL_ALWAYS_SOFTWARE", OsString::from("0")),
            ("GALLIUM_DRIVER", OsString::from("llvmpipe")),
        ]);
        let mut command = Command::new("python");
        configure_colab_render_environment(&mut command, true, |name| {
            environment.get(name).cloned()
        });

        let configured_names = command
            .get_envs()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(!configured_names.contains(&"LIBGL_ALWAYS_SOFTWARE".to_owned()));
        assert!(!configured_names.contains(&"GALLIUM_DRIVER".to_owned()));
        assert!(configured_names.contains(&"COSMOL_VIEWER_OFFSCREEN_TRACE".to_owned()));
    }
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

/// Input variants drive both runtime extraction and official type annotations.
#[derive(FromPyObject)]
pub enum BackgroundInput {
    Color(ColorInput),
    RgbaBytes([i64; 4]),
    RgbaFloat([f64; 4]),
}

impl BackgroundInput {
    fn into_background(self) -> PyResult<StaticImageBackground> {
        match self {
            Self::Color(ColorInput::Hex(value)) => {
                match value.trim().to_ascii_lowercase().as_str() {
                    "scene" | "default" => return Ok(StaticImageBackground::Scene),
                    "transparent" | "none" => return Ok(StaticImageBackground::Color([0.0; 4])),
                    _ => {}
                }
                if let Some(color) = parse_hex_background(&value)? {
                    return Ok(StaticImageBackground::Color(color));
                }
                let color = ColorInput::Hex(value).into_color()?.0;
                Ok(StaticImageBackground::Color([
                    color.x, color.y, color.z, 1.0,
                ]))
            }
            Self::Color(color) => {
                let color = color.into_color()?.0;
                Ok(StaticImageBackground::Color([
                    color.x, color.y, color.z, 1.0,
                ]))
            }
            Self::RgbaBytes(values) => Ok(StaticImageBackground::Color([
                py_color_channel_to_f32(values[0])?,
                py_color_channel_to_f32(values[1])?,
                py_color_channel_to_f32(values[2])?,
                py_color_channel_to_f32(values[3])?,
            ])),
            Self::RgbaFloat(values) => Ok(StaticImageBackground::Color([
                py_float_channel_to_f32(values[0])?,
                py_float_channel_to_f32(values[1])?,
                py_float_channel_to_f32(values[2])?,
                py_float_channel_to_f32(values[3])?,
            ])),
        }
    }
}

fn image_background(background: Option<BackgroundInput>) -> PyResult<StaticImageBackground> {
    background.map_or(
        Ok(StaticImageBackground::Scene),
        BackgroundInput::into_background,
    )
}

fn parse_hex_background(value: &str) -> PyResult<Option<[f32; 4]>> {
    let hex = value.trim().trim_start_matches('#');
    if !hex.is_ascii() || (hex.len() != 6 && hex.len() != 8) {
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

fn absolutize_output_path(path: &Path) -> PyResult<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| PyRuntimeError::new_err(format!("Error resolving output path: {err}")))
    }
}
