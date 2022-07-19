mod as_bind_group;
mod extract_resource;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;

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
    as_bind_group::derive_as_bind_group(input)
}
