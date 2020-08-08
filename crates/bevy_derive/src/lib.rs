extern crate proc_macro;

mod app_plugin;
mod as_vertex_buffer_descriptor;
mod bytes;
mod modules;
mod render_resource;
mod render_resources;
mod resource;
mod shader_defs;

use proc_macro::TokenStream;

#[proc_macro_derive(FromResources, attributes(as_crate))]
pub fn derive_from_resources(input: TokenStream) -> TokenStream {
    resource::derive_from_resources(input)
}

#[proc_macro_derive(Bytes, attributes(as_crate))]
pub fn derive_bytes(input: TokenStream) -> TokenStream {
    bytes::derive_bytes(input)
}

#[proc_macro_derive(RenderResources, attributes(render_resources, as_crate))]
pub fn derive_render_resources(input: TokenStream) -> TokenStream {
    render_resources::derive_render_resources(input)
}

#[proc_macro_derive(RenderResource, attributes(as_crate))]
pub fn derive_render_resource(input: TokenStream) -> TokenStream {
    render_resource::derive_render_resource(input)
}

#[proc_macro_derive(ShaderDefs, attributes(shader_def, as_crate))]
pub fn derive_shader_defs(input: TokenStream) -> TokenStream {
    shader_defs::derive_shader_defs(input)
}

#[proc_macro_derive(AsVertexBufferDescriptor, attributes(vertex, as_crate))]
pub fn derive_as_vertex_buffer_descriptor(input: TokenStream) -> TokenStream {
    as_vertex_buffer_descriptor::derive_as_vertex_buffer_descriptor(input)
}

#[proc_macro_derive(DynamicPlugin)]
pub fn derive_app_plugin(input: TokenStream) -> TokenStream {
    app_plugin::derive_dynamic_plugin(input)
}
