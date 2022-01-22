mod render_ecs_resource;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_render")
}

#[proc_macro_derive(ExtractResource)]
pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    render_ecs_resource::derive_extract_resource(input)
}
