//! Python projections of the native viewer, with separate host adapters.
use pyo3::prelude::*;

mod animation;
#[path = "../framework/external.rs"]
mod external;
mod image;
mod input;
mod notebook;
mod scene;
mod selection;
mod shapes;
mod styles;
mod viewer;

pub use animation::Animation;
pub use scene::Scene;
pub use viewer::Viewer;

/// Return the JSON API contract, including language names and platform restrictions.

#[pyfunction]
pub fn binding_contract_json() -> String {
    cosmol_viewer_core::binding_contract::to_json()
}

#[pymodule]
mod cosmol_viewer {
    #[pymodule_export]
    use super::selection::{Selection, Selector, select};
    #[pymodule_export]
    use super::shapes::{PySphere, PyStick};
    #[pymodule_export]
    use super::styles::{BallAndStick, Cartoon, SphereStyle, StickStyle, Surface};
    #[pymodule_export]
    use super::{Animation, Scene, Viewer, binding_contract_json};
}
