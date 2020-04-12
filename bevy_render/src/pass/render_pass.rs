use crate::{
    pipeline::PipelineDescriptor,
    render_resource::{RenderResource, RenderResourceAssignments},
    renderer_2::RenderContext,
};
use std::ops::Range;

pub trait RenderPass {
    fn get_render_context(&self) -> &dyn RenderContext;
    fn get_pipeline_descriptor(&self) -> &PipelineDescriptor;
    fn set_index_buffer(&mut self, resource: RenderResource, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, resource: RenderResource, offset: u64);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
    fn set_render_resources(
        &mut self,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>>;
}
