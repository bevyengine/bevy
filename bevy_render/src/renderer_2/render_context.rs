use crate::{
    render_resource::{
        BufferInfo, RenderResource, RenderResources, ResourceInfo,
    },
    texture::{SamplerDescriptor, TextureDescriptor, Texture}, mesh::Mesh,
};
use bevy_asset::Handle;

pub trait RenderContext {
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource;
    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
    ) -> RenderResource;
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource;
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderContext),
    ) -> RenderResource;
    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource;
    fn remove_buffer(&mut self, resource: RenderResource);
    fn remove_texture(&mut self, resource: RenderResource);
    fn remove_sampler(&mut self, resource: RenderResource);
    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo>;
    fn get_local_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo>;
    fn local_render_resources(&self) -> &RenderResources;
    fn local_render_resources_mut(&mut self) -> &mut RenderResources;
    fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource>;
    fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource>;
    fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource>;
    fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource>;
    // fn setup_render_pipeline(
    //     &mut self,
    //     pipeline_handle: Handle<PipelineDescriptor>,
    //     pipeline_descriptor: &mut PipelineDescriptor,
    //     shader_storage: &AssetStorage<Shader>,
    // );
    // fn setup_bind_groups(
    //     &mut self,
    //     render_resource_assignments: &mut RenderResourceAssignments,
    //     pipeline_descriptor: &PipelineDescriptor,
    // );

    fn create_texture_with_data(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource;
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    );
    // fn copy_buffer_to_texture(
    //     &mut self,
    //     source_buffer: RenderResource,
    //     source_offset: u64,
    //     destination_buffer: RenderResource,
    //     destination_offset: u64,
    //     size: u64,
    // );
}
