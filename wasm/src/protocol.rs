use cosmol_viewer_core::scene::{Animation, Scene, SceneV2};
use serde::{Deserialize, Serialize};

/// One-way commands shared by the notebook sender and the WASM receiver.
#[derive(Serialize, Deserialize)]
pub enum ViewerCommand {
    UpdateScene { scene: Scene },
    CameraParameterLogging { enabled: bool },
    ShowFps { enabled: bool },
    ShowCameraParameters { enabled: bool },
}

#[derive(Deserialize)]
pub(crate) struct AnimationV2 {
    static_scene: Option<SceneV2>,
    frames: Vec<SceneV2>,
    interval: u64,
    loops: i64,
    interpolate: bool,
}

impl From<AnimationV2> for Animation {
    fn from(previous: AnimationV2) -> Self {
        Self {
            static_scene: previous.static_scene.map(Into::into),
            frames: previous.frames.into_iter().map(Into::into).collect(),
            interval: previous.interval,
            loops: previous.loops,
            interpolate: previous.interpolate,
        }
    }
}

#[derive(Deserialize)]
pub(crate) enum ViewerCommandV2 {
    UpdateScene { scene: SceneV2 },
    CameraParameterLogging { enabled: bool },
    ShowFps { enabled: bool },
    ShowCameraParameters { enabled: bool },
}

impl From<ViewerCommandV2> for ViewerCommand {
    fn from(previous: ViewerCommandV2) -> Self {
        match previous {
            ViewerCommandV2::UpdateScene { scene } => Self::UpdateScene {
                scene: scene.into(),
            },
            ViewerCommandV2::CameraParameterLogging { enabled } => {
                Self::CameraParameterLogging { enabled }
            }
            ViewerCommandV2::ShowFps { enabled } => Self::ShowFps { enabled },
            ViewerCommandV2::ShowCameraParameters { enabled } => {
                Self::ShowCameraParameters { enabled }
            }
        }
    }
}
