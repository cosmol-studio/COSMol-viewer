//! Fixed framework samples. Never shipped in the viewer package.
#[path = "../../src/input.rs"]
mod input;

use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Serialize, Tsify)]
pub struct Settings {
    pub count: u8,
    pub enabled: bool,
    pub label: Option<String>,
    pub weights: Vec<u16>,
}

impl input::TypedOutput for Settings {
    type Wire = Ts<Self>;
    fn into_wire(self) -> Result<Self::Wire, JsValue> {
        Ts::from_rust(&self).map_err(|error| js_sys::Error::new(&error.to_string()).into())
    }
}

impl input::CheckedValue for Settings {
    type Wire = Self;
    fn unwrap(value: Self) -> Self {
        value
    }
}

#[wasm_bindgen]
pub struct Probe;

#[cosmol_viewer_derive::checked_wasm]
impl Probe {
    pub fn byte(value: u8) -> u8 {
        value
    }

    pub fn round_trip(value: Settings) -> Settings {
        value
    }

    pub async fn async_round_trip(value: Settings) -> Settings {
        Self::round_trip(value)
    }
}

// Negative framework samples: unsupported values must fail compilation rather
// than fall back to any. These cfgs are only passed to the negative rustc checks.
#[cfg(converter_invalid_input)]
#[cosmol_viewer_derive::checked_wasm]
impl Probe {
    pub fn untyped_input(_value: JsValue) {}
}

#[cfg(converter_invalid_output)]
#[cosmol_viewer_derive::checked_wasm]
impl Probe {
    pub fn untyped_output() -> JsValue {
        JsValue::NULL
    }
}
