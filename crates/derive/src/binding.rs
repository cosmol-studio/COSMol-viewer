//! Viewer adaptation of COSMolKit's binding-contract approach (MIT licensed).
//! Emits metadata and exact Rust signature assertions, never binding behavior.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, Ident, LitStr, Meta, Path, ReturnType, Token, Type, TypeFnPtr, Visibility, braced,
    bracketed,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

struct Registry {
    visibility: Visibility,
    name: Ident,
    entries: Vec<Entry>,
}

struct Entry {
    semantic_id: LitStr,
    rust: Path,
    signature: TypeFnPtr,
    rust_cfg: Option<Meta>,
    receiver: Ident,
    platforms: Expr,
    python: Expr,
    javascript: Expr,
    defaults: Expr,
    notes: LitStr,
}

fn set_once<T>(slot: &mut Option<T>, value: T, key: &Ident) -> syn::Result<()> {
    if slot.replace(value).is_some() {
        return Err(syn::Error::new_spanned(key, "duplicate binding field"));
    }
    Ok(())
}

fn required<T>(value: Option<T>, field: &str, span: proc_macro2::Span) -> syn::Result<T> {
    value.ok_or_else(|| syn::Error::new(span, format!("missing binding field: {field}")))
}

impl Parse for Registry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let visibility = input.parse()?;
        input.parse::<Token![static]>()?;
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let content;
        bracketed!(content in input);
        let mut entries = Vec::new();
        let mut ids = HashSet::new();
        while !content.is_empty() {
            let entry: Entry = content.parse()?;
            if !ids.insert(entry.semantic_id.value()) {
                return Err(syn::Error::new_spanned(
                    &entry.semantic_id,
                    "duplicate semantic_id",
                ));
            }
            entries.push(entry);
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }
        if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        }
        Ok(Self {
            visibility,
            name,
            entries,
        })
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        braced!(content in input);
        let span = content.span();
        let (mut semantic_id, mut rust, mut signature, mut rust_cfg, mut receiver) =
            (None, None, None, None, None);
        let (mut platforms, mut python, mut javascript, mut defaults, mut notes) =
            (None, None, None, None, None);
        while !content.is_empty() {
            let key = content.call(Ident::parse_any)?;
            content.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "semantic_id" => set_once(&mut semantic_id, content.parse()?, &key)?,
                "rust" => set_once(&mut rust, content.parse()?, &key)?,
                "signature" => set_once(&mut signature, content.parse()?, &key)?,
                "rust_cfg" => set_once(&mut rust_cfg, content.parse()?, &key)?,
                "receiver" => set_once(&mut receiver, content.parse()?, &key)?,
                "platforms" => set_once(&mut platforms, content.parse()?, &key)?,
                "python" => set_once(&mut python, content.parse()?, &key)?,
                "javascript" => set_once(&mut javascript, content.parse()?, &key)?,
                "defaults" => set_once(&mut defaults, content.parse()?, &key)?,
                "notes" => set_once(&mut notes, content.parse()?, &key)?,
                _ => return Err(syn::Error::new_spanned(key, "unknown binding field")),
            }
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }
        let entry = Self {
            semantic_id: required(semantic_id, "semantic_id", span)?,
            rust: required(rust, "rust", span)?,
            signature: required(signature, "signature", span)?,
            rust_cfg,
            receiver: required(receiver, "receiver", span)?,
            platforms: required(platforms, "platforms", span)?,
            python: required(python, "python", span)?,
            javascript: required(javascript, "javascript", span)?,
            defaults: defaults.unwrap_or_else(|| syn::parse_quote!(&[])),
            notes: notes.unwrap_or_else(|| LitStr::new("", span)),
        };
        if entry.semantic_id.value().is_empty() {
            return Err(syn::Error::new_spanned(
                &entry.semantic_id,
                "empty semantic_id",
            ));
        }
        if entry.signature.unsafety.is_some()
            || entry.signature.variadic.is_some()
            || entry.signature.abi.is_some()
        {
            return Err(syn::Error::new_spanned(
                &entry.signature,
                "signature must be a safe Rust function",
            ));
        }
        let first = entry.signature.inputs.first().map(|input| &input.ty);
        let matches_receiver = match (entry.receiver.to_string().as_str(), first) {
            ("none", _) => true,
            ("shared", Some(Type::Reference(reference))) => reference.mutability.is_none(),
            ("mutable", Some(Type::Reference(reference))) => reference.mutability.is_some(),
            ("owned", Some(Type::Path(_))) => true,
            _ => false,
        };
        if !matches_receiver {
            return Err(syn::Error::new_spanned(
                &entry.receiver,
                "receiver disagrees with signature",
            ));
        }
        Ok(entry)
    }
}

pub(crate) fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let Registry {
        visibility,
        name,
        entries,
    } = syn::parse2(input)?;
    let mut rows = Vec::new();
    let mut assertions = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Entry {
            semantic_id,
            rust,
            signature,
            rust_cfg,
            receiver,
            platforms,
            python,
            javascript,
            defaults,
            notes,
        } = entry;
        let receiver_variant = match receiver.to_string().as_str() {
            "none" => quote!(None),
            "shared" => quote!(Shared),
            "mutable" => quote!(Mutable),
            "owned" => quote!(Owned),
            _ => unreachable!(),
        };
        let skip_receiver = usize::from(receiver != "none");
        let parameters = signature.inputs.iter().skip(skip_receiver).enumerate().map(|(i, argument)| {
            let ty = &argument.ty;
            let arg_name = argument.name.as_ref().map(|(name, _)| name.to_string()).unwrap_or_else(|| format!("arg{i}"));
            quote!(crate::binding_contract::BindingParameter { name: #arg_name, type_name: stringify!(#ty) })
        });
        let output = match &signature.output {
            ReturnType::Default => quote!(()),
            ReturnType::Type(_, ty) => quote!(#ty),
        };
        rows.push(quote! {
            crate::binding_contract::BindingContractEntry {
                semantic_id: #semantic_id,
                rust_path: stringify!(#rust),
                rust_signature: stringify!(#signature),
                receiver: crate::binding_contract::BindingReceiver::#receiver_variant,
                parameters: &[#(#parameters),*],
                output_type: stringify!(#output),
                rust_platforms: #platforms,
                python: #python,
                javascript: #javascript,
                defaults: #defaults,
                notes: #notes,
            }
        });
        let assertion = format_ident!("__BINDING_ASSERT_{}_{}", name, index);
        let cfg = rust_cfg.as_ref().map(|cfg| quote!(#[cfg(#cfg)]));
        if let Some(lifetimes) = &signature.lifetimes {
            let parameters = &lifetimes.lifetimes;
            let mut instantiated = signature.clone();
            instantiated.lifetimes = None;
            assertions.push(quote! {
                #cfg const #assertion: fn() = || {
                    fn assert_signature<#parameters>() { let _: #instantiated = #rust; }
                };
            });
        } else {
            assertions.push(quote!(#cfg const #assertion: #signature = #rust;));
        }
    }
    Ok(quote! {
        #visibility static #name: &[crate::binding_contract::BindingContractEntry] = &[#(#rows),*];
        #(#assertions)*
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> TokenStream {
        quote!({ semantic_id: "Viewer.keep_alive", rust: crate::Viewer::keep_alive,
            signature: fn(viewer: crate::Viewer) -> std::io::Result<()>, receiver: owned,
            platforms: &[Native], python: Projection::supported("Viewer.keep_alive", &[Native]),
            javascript: Projection::unsupported("Native process lifecycle only") })
    }

    #[test]
    fn emits_exact_signature_assertions_and_keeps_unsupported_projection() {
        let row = entry();
        let generated = expand(quote!(pub static CONTRACT = [#row];))
            .unwrap()
            .to_string();
        assert!(generated.contains("__BINDING_ASSERT_CONTRACT_0"));
        assert!(generated.contains("Native process lifecycle only"));
        assert!(generated.contains("rust_signature"));
    }

    #[test]
    fn rejects_duplicate_semantic_ids() {
        let row = entry();
        assert!(expand(quote!(pub static CONTRACT = [#row, #row];)).is_err());
    }

    #[test]
    fn rejects_invalid_receiver_and_unknown_fields() {
        let row = entry().to_string();
        assert!(
            expand(
                format!(
                    "pub static CONTRACT = [{}];",
                    row.replace("receiver : owned", "receiver : mutable")
                )
                .parse()
                .unwrap()
            )
            .is_err()
        );
        assert!(
            expand(
                format!(
                    "pub static CONTRACT = [{}];",
                    row.replace("semantic_id", "semantic_typo")
                )
                .parse()
                .unwrap()
            )
            .is_err()
        );
    }
}
