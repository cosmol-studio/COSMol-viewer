use crate::protocol::ViewerCommand;
use crate::utils::encode_payload;
use cosmol_viewer_core::BUILD_ID;
use cosmol_viewer_core::scene::{Animation, Scene};

use pyo3::{PyErr, PyResult, Python};

pub struct NotebookViewer {
    pub id: String,
}

impl NotebookViewer {
    pub fn render(py: Python, scene: &Scene, width: f32, height: f32) -> PyResult<Self> {
        use pyo3::types::PyAnyMethods;
        use uuid::Uuid;

        let unique_id = format!("cosmol_viewer_{}", Uuid::new_v4());

        let html_code = format!(
            r#"
<canvas id="{id}" width="{width}" height="{height}" style="width:{width}px; height:{height}px;"></canvas>
            "#,
            id = unique_id,
            width = width,
            height = height
        );

        let payload = encode_payload(scene)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let escaped = serde_json::to_string(&payload).unwrap();

        let combined_js = format!(
            r#"
(function() {{
    const ns = "cosmol_viewer_{BUILD_ID}";
    console.error(ns);

    import(window[ns + "_blob_url"]).then(async (mod) => {{
        await mod.default(window[ns + "_wasm_bytes"]);

        const canvas = document.getElementById('{id}');
        const gl = canvas.getContext('webgl2', {{ alpha: true, antialias: true }});
        if (!gl) {{
            console.error("WebGL2 not supported or failed to initialize");
            return;
        }}
        const scene_payload = {SCENE};
        const app = await mod.Viewer.renderNotebook(canvas.id, scene_payload);

        window[ns + "_instances"] = window[ns + "_instances"] || {{}};
        window[ns + "_instances"]["{id}"] = app;
        console.log("Cosmol viewer instance {id} (v{BUILD_ID}) started");
    }});
}})();
    "#,
            BUILD_ID = BUILD_ID,
            id = unique_id,
            SCENE = escaped
        );

        let ipython = py.import("IPython.display")?;
        let display = ipython.getattr("display")?;

        let html = ipython
            .getattr("HTML")
            .unwrap()
            .call1((html_code,))
            .unwrap();
        display.call1((html,))?;

        let js = ipython
            .getattr("Javascript")
            .unwrap()
            .call1((combined_js,))
            .unwrap();
        display.call1((js,))?;

        Ok(Self { id: unique_id })
    }

    pub fn initiate_viewer_and_play(
        py: Python,
        animation: Animation,
        width: f32,
        height: f32,
    ) -> PyResult<Self> {
        use pyo3::types::PyAnyMethods;
        use uuid::Uuid;

        let unique_id = format!("cosmol_viewer_{}", Uuid::new_v4());

        let html_code = format!(
            r#"
<canvas id="{id}" width="{width}" height="{height}" style="width:{width}px; height:{height}px;"></canvas>
            "#,
            id = unique_id,
            width = width,
            height = height
        );

        let payload = encode_payload(&animation)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let escaped = serde_json::to_string(&payload).unwrap();

        let combined_js = format!(
            r#"
(function() {{
    const ns = "cosmol_viewer_{BUILD_ID}";

    import(window[ns + "_blob_url"]).then(async (mod) => {{
        await mod.default(window[ns + "_wasm_bytes"]);

        const canvas = document.getElementById('{id}');
        const gl = canvas.getContext('webgl2', {{ alpha: true, antialias: true }});
        if (!gl) {{
            console.error("WebGL2 not supported or failed to initialize");
            return;
        }}
        const animation_payload = {ANIMATION};
        const app = await mod.Viewer.playNotebook(canvas.id, animation_payload);

        window[ns + "_instances"] = window[ns + "_instances"] || {{}};
        window[ns + "_instances"]["{id}"] = app;
        console.log("Cosmol viewer instance {id} (v{BUILD_ID}) started");
    }});
}})();
    "#,
            BUILD_ID = BUILD_ID,
            id = unique_id,
            ANIMATION = escaped
        );
        let ipython = py.import("IPython.display")?;
        let display = ipython.getattr("display")?;

        let html = ipython.getattr("HTML")?.call1((html_code,))?;
        display.call1((html,))?;

        let js = ipython.getattr("Javascript")?.call1((combined_js,))?;
        display.call1((js,))?;

        Ok(Self { id: unique_id })
    }

    /// Submits a typed command without waiting for a browser response.
    pub fn send(&self, py: Python, command: &ViewerCommand) -> PyResult<()> {
        use pyo3::types::PyAnyMethods;

        let escaped = serde_json::to_string::<String>(
            &encode_payload(command)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        )
        .unwrap();
        let combined_js = format!(
            r#"
(async function() {{
    const ns = "cosmol_viewer_{BUILD_ID}";
    const instances = window[ns + "_instances"] || {{}};
    const app = instances["{id}"];
    if (app) {{
        try {{
            await app.dispatch({escaped});
        }} catch (err) {{
            console.error("Error dispatching command on instance {id} (v{BUILD_ID}):", err);
        }}
    }} else {{
        console.error("No app found for ID {id} in namespace", ns);
    }}
}})();
        "#,
            BUILD_ID = BUILD_ID,
            id = self.id,
            escaped = escaped
        );

        let ipython = py.import("IPython.display")?;
        let display = ipython.getattr("display")?;

        let js = ipython.getattr("Javascript")?.call1((combined_js,))?;
        display.call1((js,))?;
        Ok(())
    }

    pub fn update(&self, py: Python, scene: &Scene) -> PyResult<()> {
        self.send(
            py,
            &ViewerCommand::UpdateScene {
                scene: scene.clone(),
            },
        )
    }

    pub fn camera_parameter_logging(&self, py: Python, enabled: bool) -> PyResult<()> {
        self.send(py, &ViewerCommand::CameraParameterLogging { enabled })
    }

    pub fn show_fps(&self, py: Python, enabled: bool) -> PyResult<()> {
        self.send(py, &ViewerCommand::ShowFps { enabled })
    }

    pub fn show_camera_parameters(&self, py: Python, enabled: bool) -> PyResult<()> {
        self.send(py, &ViewerCommand::ShowCameraParameters { enabled })
    }
}

pub trait JsBridge {
    fn update(scene: &Scene) -> ();
}
