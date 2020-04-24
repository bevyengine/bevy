use super::RenderResourceContext;
use crate::{
    pass::{PassDescriptor, RenderPass},
    pipeline::{BindGroupDescriptor, PipelineDescriptor},
    render_resource::{RenderResource, RenderResourceAssignments, RenderResourceSetId},
    shader::Shader,
    texture::TextureDescriptor,
};
use bevy_asset::{AssetStorage, Handle};

pub trait RenderContext {
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
    fn create_bind_group(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<RenderResourceSetId>;
    fn create_render_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
        shader_storage: &AssetStorage<Shader>,
    );
    fn setup_bind_groups(
        &mut self,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            self.create_bind_group(bind_group, render_resource_assignments);
        }
    }
    fn begin_pass(
        &mut self,
        pass_descriptor: &PassDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
        run_pass: &mut dyn Fn(&mut dyn RenderPass),
    );
}
