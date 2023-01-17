use std::collections::HashMap;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Error};

static CRATE_LOOKUP: once_cell::sync::Lazy<
    HashMap<&'static str, (&'static str, Option<&'static str>)>,
> = once_cell::sync::Lazy::new(|| HashMap::from([("Camera", ("bevy_render", Some("camera")))]));

pub fn derive_pairs_with_others(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    #[allow(unused)]
    let component_name = &ast.ident;
    let trait_name = format_ident!("PairsWith{}", ast.ident);

    TokenStream::from(quote! {
        /// Lists component types that can be added to entities with the given component.
        /// Note: in the bevy project this will only show components included in with engine. If
        /// your project uses plugins that add to this list, they will be visible in the
        /// bevy documentation in your project tree.
        pub trait #trait_name : Component {}
    })
}

pub fn derive_pairs_with(attrs: TokenStream, mut item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attrs as AttributeArgs);

    let mut pairs_with = None;
    for meta in attrs {
        use syn::NestedMeta::{Lit, Meta};
        match meta {
            Meta(syn::Meta::Path(path)) => {
                if pairs_with.is_some() {
                    return Error::new_spanned(
                        path,
                        "multiple parse_with attributes not supported",
                    )
                    .into_compile_error()
                    .into();
                }
                pairs_with = Some(path.get_ident().unwrap().to_owned());
            }
            Lit(tok) => {
                return Error::new_spanned(tok, "unexpected token in parse_with attribute")
                    .into_compile_error()
                    .into();
            }
            Meta(tok) => {
                return Error::new_spanned(tok, "unexpected token in parse_with attribute")
                    .into_compile_error()
                    .into();
            }
        }
    }

    let pairs_with = pairs_with.unwrap();
    let pairs_with_string = pairs_with.to_string();
    let (crate_path, subcrate) = CRATE_LOOKUP.get(pairs_with_string.as_str()).unwrap();
    let crate_path = BevyManifest::get_path_direct(crate_path);
    let crate_path = match subcrate {
        Some(subcrate) => {
            let subcrate: syn::Path = syn::parse(subcrate.parse::<TokenStream>().unwrap()).unwrap();
            quote! { #crate_path::#subcrate }
        }
        None => {
            quote! { crate_path }
        }
    };
    let trait_name = format_ident!("PairsWith{}", pairs_with);

    let ast = item.clone();
    let ast = parse_macro_input!(ast as DeriveInput);
    let component_name = &ast.ident;

    item.extend(TokenStream::from(quote! {
         impl #crate_path::#trait_name for #component_name {}
    }));
    item
}
