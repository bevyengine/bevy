extern crate proc_macro;

mod app_plugin;
mod as_vertex_buffer_descriptor;
mod attributes;
mod bytes;
mod component_set;
mod modules;
mod render_resource;
mod render_resources;
mod resource;
mod shader_defs;

use proc_macro::TokenStream;

#[proc_macro_derive(FromResources, attributes(module))]
pub fn derive_from_resources(input: TokenStream) -> TokenStream {
    resource::derive_from_resources(input)
}

#[proc_macro_derive(Bytes, attributes(module))]
pub fn derive_bytes(input: TokenStream) -> TokenStream {
    bytes::derive_bytes(input)
}

#[proc_macro_derive(RenderResources, attributes(render_resources, module))]
pub fn derive_render_resources(input: TokenStream) -> TokenStream {
    render_resources::derive_render_resources(input)
}

#[proc_macro_derive(RenderResource, attributes(module))]
pub fn derive_render_resource(input: TokenStream) -> TokenStream {
    render_resource::derive_render_resource(input)
}

#[proc_macro_derive(ShaderDefs, attributes(shader_def, module))]
pub fn derive_shader_defs(input: TokenStream) -> TokenStream {
    shader_defs::derive_shader_defs(input)
}

#[proc_macro_derive(AsVertexBufferDescriptor, attributes(vertex, module))]
pub fn derive_as_vertex_buffer_descriptor(input: TokenStream) -> TokenStream {
    as_vertex_buffer_descriptor::derive_as_vertex_buffer_descriptor(input)
}

#[proc_macro_derive(ComponentSet, attributes(tag, module))]
pub fn derive_component_set(input: TokenStream) -> TokenStream {
    component_set::derive_component_set(input)
}

#[proc_macro_derive(DynamicAppPlugin)]
pub fn derive_app_plugin(input: TokenStream) -> TokenStream {
    app_plugin::derive_dynamic_app_plugin(input)
}
