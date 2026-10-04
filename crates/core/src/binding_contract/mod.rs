//! Cross-language API contracts. Registration is not binding generation.
//! Platform availability is separate from language: Python can host notebooks.
//!
//! Registering the wrong Rust signature fails to compile:
//!
//! ```compile_fail,E0308
//! use cosmol_viewer_core::binding_contract;
//! use cosmol_viewer_derive::binding_contract;
//! use binding_contract::{BindingProjection as Projection, RuntimePlatform::Native};
//! binding_contract! {
//!     static WRONG = [{
//!         semantic_id: "Scene.new", rust: cosmol_viewer_core::scene::Scene::new,
//!         signature: fn() -> usize, receiver: none, platforms: &[Native],
//!         python: Projection::unsupported("test"),
//!         javascript: Projection::unsupported("test"),
//!     }];
//! }
//! ```

pub use cosmol_viewer_derive::binding_contract;
use serde::Serialize;

mod registry;
pub use registry::BINDING_CONTRACT;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePlatform {
    Native,
    Notebook,
    Browser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingReceiver {
    None,
    Shared,
    Mutable,
    Owned,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct BindingProjection {
    pub name: Option<&'static str>,
    pub platforms: &'static [RuntimePlatform],
    pub unsupported_reason: Option<&'static str>,
    pub command: Option<&'static str>,
    /// Optional notebook transport path, separate from the normal language API.
    pub notebook_endpoint: Option<&'static str>,
}

impl BindingProjection {
    pub const fn supported(name: &'static str, platforms: &'static [RuntimePlatform]) -> Self {
        assert!(!name.is_empty() && !platforms.is_empty());
        Self {
            name: Some(name),
            platforms,
            unsupported_reason: None,
            command: None,
            notebook_endpoint: None,
        }
    }

    pub const fn with_notebook(
        self,
        endpoint: &'static str,
        command: Option<&'static str>,
    ) -> Self {
        assert!(!endpoint.is_empty() && self.name.is_some());
        Self {
            command,
            notebook_endpoint: Some(endpoint),
            ..self
        }
    }

    pub const fn unsupported(reason: &'static str) -> Self {
        assert!(!reason.is_empty());
        Self {
            name: None,
            platforms: &[],
            unsupported_reason: Some(reason),
            command: None,
            notebook_endpoint: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct BindingParameter {
    pub name: &'static str,
    pub type_name: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct BindingDefault {
    pub parameter: &'static str,
    pub python: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct BindingContractEntry {
    pub semantic_id: &'static str,
    pub rust_path: &'static str,
    pub rust_signature: &'static str,
    pub receiver: BindingReceiver,
    pub parameters: &'static [BindingParameter],
    pub output_type: &'static str,
    pub rust_platforms: &'static [RuntimePlatform],
    pub python: BindingProjection,
    pub javascript: BindingProjection,
    pub defaults: &'static [BindingDefault],
    pub notes: &'static str,
}

pub fn to_json() -> String {
    serde_json::to_string(BINDING_CONTRACT)
        .expect("binding contracts contain only serializable metadata")
}
