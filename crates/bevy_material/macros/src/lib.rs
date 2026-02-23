#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use bevy_macro_utils::{derive_label, BevyManifest};
use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn bevy_material_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_material"))
}

#[proc_macro_derive(ShaderLabel)]
pub fn derive_shader_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_material_path();
    trait_path.segments.push(format_ident!("labels").into());
    trait_path
        .segments
        .push(format_ident!("ShaderLabel").into());
    derive_label(input, "ShaderLabel", &trait_path)
}

#[proc_macro_derive(DrawFunctionLabel)]
pub fn derive_draw_function_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_material_path();
    trait_path.segments.push(format_ident!("labels").into());
    trait_path
        .segments
        .push(format_ident!("DrawFunctionLabel").into());
    derive_label(input, "DrawFunctionLabel", &trait_path)
}
