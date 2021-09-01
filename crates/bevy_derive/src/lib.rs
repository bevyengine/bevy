extern crate proc_macro;

mod app_plugin;
mod bevy_main;
mod bytes;
mod enum_variant_meta;
mod modules;
mod render_resource;
mod render_resources;
mod shader_defs;

use proc_macro::TokenStream;

/// Derives the Bytes trait. Each field must also implements Bytes or this will fail.
#[proc_macro_derive(Bytes)]
pub fn derive_bytes(input: TokenStream) -> TokenStream {
    bytes::derive_bytes(input)
}

/// Derives the RenderResources trait. Each field must implement RenderResource or this will fail.
/// You can ignore fields using `#[render_resources(ignore)]`.
#[proc_macro_derive(RenderResources, attributes(render_resources))]
pub fn derive_render_resources(input: TokenStream) -> TokenStream {
    render_resources::derive_render_resources(input)
}

/// Derives the RenderResource trait. The type must also implement `Bytes` or this will fail.
#[proc_macro_derive(RenderResource)]
pub fn derive_render_resource(input: TokenStream) -> TokenStream {
    render_resource::derive_render_resource(input)
}

/// Derives the ShaderDefs trait. Each field must implement ShaderDef or this will fail.
/// You can ignore fields using `#[shader_defs(ignore)]`.
#[proc_macro_derive(ShaderDefs, attributes(shader_def))]
pub fn derive_shader_defs(input: TokenStream) -> TokenStream {
    shader_defs::derive_shader_defs(input)
}

/// Generates a dynamic plugin entry point function for the given `Plugin` type.  
#[proc_macro_derive(DynamicPlugin)]
pub fn derive_dynamic_plugin(input: TokenStream) -> TokenStream {
    app_plugin::derive_dynamic_plugin(input)
}

#[proc_macro_attribute]
pub fn bevy_main(attr: TokenStream, item: TokenStream) -> TokenStream {
    bevy_main::bevy_main(attr, item)
}

#[proc_macro_derive(EnumVariantMeta)]
pub fn derive_enum_variant_meta(input: TokenStream) -> TokenStream {
    enum_variant_meta::derive_enum_variant_meta(input)
}
