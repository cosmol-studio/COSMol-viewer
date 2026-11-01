//! Shared checked value boundary. No viewer types or method-specific policies.
use serde::{Deserialize, de::DeserializeOwned};
use tsify::{Ts, Tsify};
use wasm_bindgen::{JsCast, JsValue};

/// Successful outputs must retain a concrete generated type. Raw JsValue is
/// deliberately not an output projection; it remains available for errors.
pub trait TypedOutput: Sized {
    type Wire;
    fn into_wire(self) -> Result<Self::Wire, JsValue>;
}

/// Values already represented by the official wasm-bindgen ABI.
pub trait NativeOutput {}

impl<T: NativeOutput> TypedOutput for T {
    type Wire = Self;
    fn into_wire(self) -> Result<Self, JsValue> {
        Ok(self)
    }
}

macro_rules! outputs {
    ($($ty:ty),* $(,)?) => { $(impl NativeOutput for $ty {})* };
}
outputs!(
    (),
    bool,
    String,
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
    f32,
    f64
);
impl<T: NativeOutput> NativeOutput for Vec<T> {}
impl<T: NativeOutput> NativeOutput for Option<T> {}
impl<T: Tsify> NativeOutput for Ts<T> {}

/// A native value declares its wire representation once. Business types own
/// their own implementations; primitive associations live here.
pub trait CheckedValue: Sized {
    type Wire: Tsify + DeserializeOwned;
    fn unwrap(value: Self::Wire) -> Self;
    fn check_representation(_value: &JsValue) -> Result<(), JsValue> {
        Ok(())
    }
}

pub fn read<T: CheckedValue>(value: Ts<T::Wire>) -> Result<T, JsValue>
where
    <T::Wire as Tsify>::JsType: Clone,
{
    T::check_representation(&value.js_value())?;
    let decoded = value
        .to_rust()
        .map_err(|error| js_sys::TypeError::new(&error.to_string()))?;
    Ok(T::unwrap(decoded))
}

macro_rules! scalar {
    ($name:ident, $native:ty) => {
        #[derive(Deserialize, Tsify)]
        #[serde(transparent)]
        pub struct $name($native);

        impl CheckedValue for $native {
            type Wire = $name;
            fn unwrap(value: Self::Wire) -> Self {
                value.0
            }
        }
    };
}

scalar!(FloatInput, f32);
scalar!(DoubleInput, f64);
scalar!(ByteInput, u8);
scalar!(U16Input, u16);
scalar!(U32Input, u32);
scalar!(I8Input, i8);
scalar!(I16Input, i16);
scalar!(I32Input, i32);
scalar!(IndexInput, usize);
scalar!(SignedIndexInput, isize);
scalar!(BoolInput, bool);
scalar!(TextInput, String);

#[derive(Deserialize, Tsify)]
#[serde(transparent)]
pub struct IntegerInput(#[tsify(type = "bigint")] i64);

impl CheckedValue for i64 {
    type Wire = IntegerInput;
    fn unwrap(value: Self::Wire) -> Self {
        value.0
    }
    fn check_representation(value: &JsValue) -> Result<(), JsValue> {
        if value.is_bigint() {
            Ok(())
        } else {
            Err(js_sys::TypeError::new("expected bigint").into())
        }
    }
}

#[derive(Deserialize, Tsify)]
#[serde(transparent)]
pub struct UnsignedIntegerInput(#[tsify(type = "bigint")] u64);

impl CheckedValue for u64 {
    type Wire = UnsignedIntegerInput;
    fn unwrap(value: Self::Wire) -> Self {
        value.0
    }
    fn check_representation(value: &JsValue) -> Result<(), JsValue> {
        <i64 as CheckedValue>::check_representation(value)
    }
}

macro_rules! typed_array {
    ($name:ident, $element:ty, $array:ident, $typescript:literal) => {
        #[derive(Deserialize, Tsify)]
        #[serde(transparent)]
        pub struct $name(#[tsify(type = $typescript)] Vec<$element>);

        impl CheckedValue for Vec<$element> {
            type Wire = $name;
            fn unwrap(value: Self::Wire) -> Self {
                value.0
            }
            fn check_representation(value: &JsValue) -> Result<(), JsValue> {
                if value.is_instance_of::<js_sys::$array>() {
                    Ok(())
                } else {
                    Err(js_sys::TypeError::new(concat!("expected ", $typescript)).into())
                }
            }
        }
    };
}

typed_array!(FloatArrayInput, f32, Float32Array, "Float32Array");
typed_array!(BytesInput, u8, Uint8Array, "Uint8Array");
