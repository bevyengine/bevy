use crate::{
    render_resource::{
        BufferInfo, RenderResource, AssetResources, ResourceInfo,
    },
    texture::{SamplerDescriptor, TextureDescriptor, Texture}, mesh::Mesh,
};
use bevy_asset::Handle;

pub struct GlobalRenderResourceContext {
    pub context: Box<dyn RenderResourceContext + Send + Sync + 'static>,
}

// TODO: Rename to RenderResources after cleaning up AssetResources rename
pub trait RenderResourceContext {
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource;
    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
    ) -> RenderResource;
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource;
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderResourceContext),
    ) -> RenderResource;
    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource;
    fn remove_buffer(&mut self, resource: RenderResource);
    fn remove_texture(&mut self, resource: RenderResource);
    fn remove_sampler(&mut self, resource: RenderResource);
    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo>;
    fn asset_resources(&self) -> &AssetResources;
    fn asset_resources_mut(&mut self) -> &mut AssetResources;
    fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource>;
    fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource>;
    fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource>;
    fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource>;
}
