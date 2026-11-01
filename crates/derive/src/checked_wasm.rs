//! Generate checked JS adapters from native declarations, without business metadata.
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ItemImpl, Pat, ReturnType, Type, Visibility};

pub(crate) fn expand(native: ItemImpl) -> syn::Result<TokenStream> {
    if native.trait_.is_some() || !native.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &native,
            "expected a concrete inherent impl",
        ));
    }
    let owner = &native.self_ty;
    let mut adapters = Vec::new();
    for item in &native.items {
        let ImplItem::Fn(method) = item else { continue };
        if !matches!(method.vis, Visibility::Public(_)) {
            continue;
        }
        let signature = &method.sig;
        if !signature.generics.params.is_empty()
            || matches!(signature.safety, syn::Safety::Unsafe(_))
        {
            return Err(syn::Error::new_spanned(
                signature,
                "exported methods must be safe and concrete",
            ));
        }
        let name = &signature.ident;
        let adapter = format_ident!("__wasm_{name}");
        let mut parts = name
            .to_string()
            .split('_')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut javascript = parts.remove(0);
        for part in parts {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                javascript.extend(first.to_uppercase());
                javascript.extend(chars);
            }
        }
        let javascript = format_ident!("{javascript}");
        let mut arguments = Vec::new();
        let mut conversions = Vec::new();
        let mut forwarded = Vec::new();
        let mut receiver = false;
        for argument in &signature.inputs {
            match argument {
                FnArg::Receiver(value) => {
                    receiver = true;
                    arguments.push(quote!(#value));
                }
                FnArg::Typed(value) => {
                    let Pat::Ident(pattern) = &*value.pat else {
                        return Err(syn::Error::new_spanned(value, "expected a named argument"));
                    };
                    let identifier = &pattern.ident;
                    let ty = &value.ty;
                    if let Type::Reference(reference) = &**ty {
                        if matches!(&*reference.elem, Type::Slice(_))
                            || matches!(&*reference.elem, Type::Path(path) if path.path.is_ident("str"))
                        {
                            return Err(syn::Error::new_spanned(
                                ty,
                                "value inputs must use an owned CheckedValue type; references are for native handles",
                            ));
                        }
                        // Concrete native class borrows retain wasm-bindgen's pointer validation.
                        arguments.push(quote!(#identifier: #ty));
                    } else {
                        arguments.push(quote!(#identifier: ::tsify::Ts<<#ty as crate::input::CheckedValue>::Wire>));
                        conversions.push(
                            quote!(let #identifier = crate::input::read::<#ty>(#identifier)?;),
                        );
                    }
                    forwarded.push(quote!(#identifier));
                }
            }
        }
        let call = if receiver {
            quote!(self.#name(#(#forwarded),*))
        } else {
            quote!(Self::#name(#(#forwarded),*))
        };
        let asynchronous = &signature.asyncness;
        let call = if asynchronous.is_some() {
            quote!(#call.await)
        } else {
            call
        };
        let (output, body) = match &signature.output {
            ReturnType::Default => (quote!(()), quote!(Ok(#call))),
            ReturnType::Type(_, ty) => {
                let result = if let Type::Path(path) = &**ty {
                    path.path
                        .segments
                        .last()
                        .filter(|segment| segment.ident == "Result")
                } else {
                    None
                };
                if let Some(result) = result {
                    let syn::PathArguments::AngleBracketed(args) = &result.arguments else {
                        return Err(syn::Error::new_spanned(ty, "expected Result<T, E>"));
                    };
                    let Some(syn::GenericArgument::Type(ok)) = args.args.first() else {
                        return Err(syn::Error::new_spanned(ty, "expected Result<T, E>"));
                    };
                    (
                        quote!(#ok),
                        quote!(#call.map_err(::wasm_bindgen::JsValue::from)),
                    )
                } else {
                    (quote!(#ty), quote!(Ok(#call)))
                }
            }
        };
        let docs = method
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("doc"));
        let gates = method
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("cfg"));
        adapters.push(quote! {
            #(#docs)*
            #(#gates)*
            #[wasm_bindgen::prelude::wasm_bindgen(js_name = #javascript)]
            pub #asynchronous fn #adapter(#(#arguments),*) -> Result<<#output as crate::input::TypedOutput>::Wire, ::wasm_bindgen::JsValue> {
                #(#conversions)*
                let native: Result<#output, ::wasm_bindgen::JsValue> = #body;
                crate::input::TypedOutput::into_wire(native?)
            }
        });
    }
    Ok(quote! {
        #native
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl #owner { #(#adapters)* }
    })
}
