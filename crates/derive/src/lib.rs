use proc_macro::TokenStream;

mod binding;
mod checked_wasm;

/// Export checked value arguments through Tsify while preserving native methods.
#[proc_macro_attribute]
pub fn checked_wasm(_args: TokenStream, input: TokenStream) -> TokenStream {
    match syn::parse::<syn::ItemImpl>(input).and_then(checked_wasm::expand) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declare cross-language API metadata and compiler-checked Rust signatures.
#[proc_macro]
pub fn binding_contract(input: TokenStream) -> TokenStream {
    match binding::expand(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
