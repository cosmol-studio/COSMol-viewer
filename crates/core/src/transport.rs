//! Request/feedback transport. Port types and channel setup are backend details.

#[cfg(any(target_arch = "wasm32", test))]
mod local;
#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
pub use local::{RendererTransport, ViewerTransport, pair};
#[cfg(windows)]
pub(crate) use native::{IPC_DIRECTORY, IpcDirectory, ipc_config};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{RendererTransport, ViewerTransport};

pub type Result<T> = std::result::Result<T, String>;
