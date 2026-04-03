use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, Result, Token, parse::Parse, parse::ParseStream, parse_macro_input};

struct ColorMethodsInput {
    py_name: Ident,
    _comma: Token![,],
    ret_name: Ident,
}

impl Parse for ColorMethodsInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            py_name: input.parse()?,
            _comma: input.parse()?,
            ret_name: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn gen_color_methods_submission(input: TokenStream) -> TokenStream {
    let ColorMethodsInput {
        py_name, ret_name, ..
    } = parse_macro_input!(input as ColorMethodsInput);

    let py_name_str = py_name.to_string();
    let ret_name_str = ret_name.to_string();

    let stub = format!(
        r#"class {py_name}:
    from typing import overload, Self

    @overload
    def color(self, c: tuple[int, int, int]) -> {ret_name}: ...

    @overload
    def color(self, c: str) -> {ret_name}: ...
"#,
        py_name = py_name_str,
        ret_name = ret_name_str,
    );

    let stub_lit = syn::LitStr::new(&stub, Span::call_site());

    quote! {
        submit! {
            gen_methods_from_python! {
                #stub_lit
            }
        }
    }
    .into()
}
