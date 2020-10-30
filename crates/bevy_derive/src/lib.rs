extern crate proc_macro;

mod app_plugin;
mod as_vertex_buffer_descriptor;
mod bytes;
mod modules;
mod render_resource;
mod render_resources;
mod resource;
mod shader_defs;
mod type_uuid;

use proc_macro::TokenStream;

/// Derives the FromResources trait. Each field must also implement the FromResources trait or this will fail. FromResources is
/// automatically implemented for types that implement Default.
#[proc_macro_derive(FromResources, attributes(as_crate))]
pub fn derive_from_resources(input: TokenStream) -> TokenStream {
    resource::derive_from_resources(input)
}

/// Derives the Bytes trait. Each field must also implements Bytes or this will fail.
#[proc_macro_derive(Bytes, attributes(as_crate))]
pub fn derive_bytes(input: TokenStream) -> TokenStream {
    bytes::derive_bytes(input)
}

/// Derives the RenderResources trait. Each field must implement RenderResource or this will fail.
/// You can ignore fields using `#[render_resources(ignore)]`.
#[proc_macro_derive(RenderResources, attributes(render_resources, as_crate))]
pub fn derive_render_resources(input: TokenStream) -> TokenStream {
    render_resources::derive_render_resources(input)
}

/// Derives the RenderResource trait. The type must also implement `Bytes` or this will fail.
#[proc_macro_derive(RenderResource, attributes(as_crate))]
pub fn derive_render_resource(input: TokenStream) -> TokenStream {
    render_resource::derive_render_resource(input)
}

/// Derives the ShaderDefs trait. Each field must implement ShaderDef or this will fail.
/// You can ignore fields using `#[shader_defs(ignore)]`.
#[proc_macro_derive(ShaderDefs, attributes(shader_def, as_crate))]
pub fn derive_shader_defs(input: TokenStream) -> TokenStream {
    shader_defs::derive_shader_defs(input)
}

/// Derives the AsVertexBufferDescriptor trait.
#[proc_macro_derive(AsVertexBufferDescriptor, attributes(vertex, as_crate))]
pub fn derive_as_vertex_buffer_descriptor(input: TokenStream) -> TokenStream {
    as_vertex_buffer_descriptor::derive_as_vertex_buffer_descriptor(input)
}

/// Generates a dynamic plugin entry point function for the given `Plugin` type.  
#[proc_macro_derive(DynamicPlugin)]
pub fn derive_dynamic_plugin(input: TokenStream) -> TokenStream {
    app_plugin::derive_dynamic_plugin(input)
}

// From https://github.com/randomPoison/type-uuid
#[proc_macro_derive(TypeUuid, attributes(uuid))]
pub fn type_uuid_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    type_uuid::type_uuid_derive(input)
}

#[proc_macro]
pub fn external_type_uuid(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    type_uuid::external_type_uuid(tokens)
}
