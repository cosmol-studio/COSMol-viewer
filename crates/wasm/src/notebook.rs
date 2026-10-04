//! Notebook-only transport endpoints. Decode first, then use the typed Viewer API.
use crate::{
    Animation, Scene, Viewer,
    protocol::ViewerCommand,
    utils::{compress_data, decompress_animation, decompress_data},
};
impl Viewer {
    /// Internal notebook startup: payload is not the public JS scene API.
    pub async fn render_notebook(canvas_id: String, payload: String) -> Result<Self, String> {
        let scene: cosmol_viewer_core::scene::Scene =
            decompress_data(&payload).map_err(|error| error.to_string())?;
        Self::render_into(canvas_id, &Scene::from(scene)).await
    }
    /// Internal notebook animation startup, accepting the versioned transport envelope.
    pub async fn play_notebook(canvas_id: String, payload: String) -> Result<Self, String> {
        let animation: cosmol_viewer_core::scene::Animation =
            decompress_animation(&payload).map_err(|error| error.to_string())?;
        Self::play_into(canvas_id, &Animation::from(animation)).await
    }
    /// Internal one-way notebook command ingress. Ordinary JS callers use update directly.
    pub fn dispatch(&self, payload: String) -> Result<(), String> {
        let command: ViewerCommand =
            decompress_data(&payload).map_err(|error| error.to_string())?;
        match command {
            ViewerCommand::UpdateScene { scene } => self.update(&Scene::from(scene)),
            ViewerCommand::SetCameraParameterLogging { enabled } => {
                self.set_camera_parameter_logging(enabled)
            }
        }
    }
    /// Internal notebook screenshot envelope. Ordinary JS callers receive PNG bytes.
    pub async fn take_screenshot_notebook(&self) -> Result<String, String> {
        compress_data(&self.take_screenshot().await?).map_err(|error| error.to_string())
    }
}
