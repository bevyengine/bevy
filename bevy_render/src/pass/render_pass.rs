use crate::{
    pipeline::PipelineDescriptor,
    render_resource::{RenderResource, RenderResourceAssignments},
    renderer::RenderContext,
};
use bevy_asset::Handle;
use std::ops::Range;

pub trait RenderPass {
    fn get_render_context(&self) -> &dyn RenderContext;
    fn set_index_buffer(&mut self, resource: RenderResource, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, resource: RenderResource, offset: u64);
    fn set_pipeline(&mut self, pipeline_handle: Handle<PipelineDescriptor>);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
    // TODO: try to somehow take into account the "set" pipeline instead of passing it in here
    fn set_render_resources(
        &mut self,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>>;
}
