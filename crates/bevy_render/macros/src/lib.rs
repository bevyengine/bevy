mod as_bind_group;
mod extract_resource;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::default()
        .maybe_get_path("bevy_render")
        // NOTE: If the derivation is within bevy_render, then we need to return 'crate'
        .unwrap_or_else(|| BevyManifest::parse_str("crate"))
}

#[proc_macro_derive(ExtractResource)]
pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    extract_resource::derive_extract_resource(input)
}

#[proc_macro_derive(AsBindGroup, attributes(uniform, texture, sampler, bind_group_data))]
pub fn derive_as_bind_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    as_bind_group::derive_as_bind_group(input).unwrap_or_else(|err| err.to_compile_error().into())
}
