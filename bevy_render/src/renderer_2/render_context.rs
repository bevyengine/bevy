use crate::{render_resource::RenderResource, texture::TextureDescriptor};
use super::RenderResourceContext;

pub trait RenderContext {
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

    fn resources(&self) -> &dyn RenderResourceContext;
    fn resources_mut(&mut self) -> &mut dyn RenderResourceContext;

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
