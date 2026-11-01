# JavaScript binding contract

Rust owns behavior. `#[checked_wasm]` projects native method declarations through
checked Tsify inputs and wasm-bindgen. The official extractor emits JavaScript
and TypeScript from the compiled crate; Tsify derives structured declarations
from the same Serde types that perform runtime conversion.
Do not edit generated `.js` or `.d.ts`, maintain signature inventories, parse
Rust source in build scripts, or patch generated code for individual methods.

## Ownership

- `scene.rs` projects core scene state; `animation.rs` owns animation snapshots.
- `viewer.rs` owns the browser renderer lifecycle and typed request dispatch.
- `notebook.rs` decodes notebook inputs into those same typed operations.
- `protocol.rs` and `utils.rs` own notebook transport, shared with Python.
- `input.rs` owns generic checked conversions and primitive wire associations.
- `crates/derive/src/checked_wasm.rs` generates adapters from native methods. It
  does not enumerate business types, methods, errors or defaults.
- `build.py` only builds, packages and invokes checks. It must not enumerate
  business types/methods or grow when an ordinary method is added.

Add APIs in the owning Rust module with concrete parameter/return types inside
a `#[checked_wasm]` impl. Method names project to camelCase automatically.
Existing static `.new()` factories remain factories. Private helpers live in
unexported impl blocks. Binding
attributes are enabled only for browser WASM, so Python's bridge and native
model tests do not acquire a JavaScript runtime dependency.

## Values and ownership

Native state and rendering remain in the core crate. Scene/animation inputs are
borrowed handles. Frames, updates and explicit `cloneScene()` take snapshots;
merging a scene with itself must not hold two locks. Async exports return typed
Promises and keep borrowed handles alive until completion. No generated adapter
mutex is held over an await.

## Checked input and typed output

Every owned value argument crosses the ABI as `Ts<CheckedValue::Wire>`, preserving
its original JS representation until shared `input::read` validates it. Serde
checks scalar types and integer ranges. String/boolean-to-number coercion,
fractional integers and integer overflow fail with TypeError. Float values keep
the native semantics; domain checks remain with the native method.

Numeric vectors use Float32Array, RGB bytes use Uint8Array, and loop counts use
bigint. Shared representation checks enforce those exact declared kinds before
decoding. Never pre-coerce a value with Number, BigInt or a TypedArray constructor.
Vector decoding reads the full sequence; native length/domain checks happen
before mutation. All argument conversions finish before the native method runs.

Native class references keep wasm-bindgen's class/pointer validation and borrowing;
they are not serialized into replacement objects. Invalid value inputs produce
TypeError. Native operation errors follow the existing native error projection;
the adapter does not classify errors by matching messages.

Structured types derive Tsify and Serde in their owning module and implement
CheckedValue there, with no framework lookup entry. Structured outputs derive
Serialize/Tsify and implement TypedOutput once, using `Ts::from_rust` for the
associated wire value. Methods return native values directly; the adapter invokes
the converter. Do not use Tsify's deprecated `into_wasm_abi`/`from_wasm_abi`
derives, which can leak memory. Primitive outputs and
native class handles retain the official wasm-bindgen projection. No handwritten
TypeScript fragments, `any` fallback, unchecked type overrides, or per-method
validation patches are allowed. Unsupported conversions must fail compilation.
The generated adapter requires TypedOutput for successful returns; raw JsValue
is reserved for errors. Native handle types declare NativeOutput next
to their definition. Fixed negative compilation samples enforce both boundaries.

Notebook payload strings are internal transport endpoints, not replacements
for typed Scene/Animation parameters. Native-only APIs remain absent in JS.
The core registry audits semantic availability across languages; it does not
generate signatures or supply a second implementation.

## Verification

The independent fixed converter fixture exercises new structured inputs/outputs
and scalar validation using the same framework. It is never shipped in the viewer
package. The fixed TypeScript consumers check representative ownership, async results,
and rejected calls. Node tests execute the generated WASM. Chromium tests cover
render/update/play, screenshots and canvas cleanup. Extend behavioral tests for
new behavior, not a parallel list of every signature. Ordinary API additions
must not require new build machinery or per-function generation exceptions.
