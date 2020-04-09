use crate::{
    render_resource::{
        BufferInfo, RenderResource, RenderResources, ResourceInfo,
    },
    texture::{SamplerDescriptor, TextureDescriptor},
};

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
    fn get_resource_info_mut(&mut self, resource: RenderResource) -> Option<&mut ResourceInfo>;
    fn render_resources(&self) -> &RenderResources;
    fn render_resources_mut(&mut self) -> &mut RenderResources;
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
        bytes: Option<&[u8]>,
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
