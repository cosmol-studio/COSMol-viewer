use cosmol_viewer_core::scene::Scene;
use serde::{Deserialize, Serialize};

/// One-way commands shared by the notebook sender and the WASM receiver.
#[derive(Serialize, Deserialize)]
pub enum ViewerCommand {
    UpdateScene { scene: Scene },
    CameraParameterLogging { enabled: bool },
    ShowFps { enabled: bool },
    ShowCameraParameters { enabled: bool },
}
