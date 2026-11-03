//! Python notebook bootstrap; browser transport remains in the WASM crate.
use crate::viewer::RuntimeEnv;
use cosmol_viewer_core::BUILD_ID;
use pyo3::prelude::*;
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
