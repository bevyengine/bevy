use crate::{
    pipeline::PipelineDescriptor,
    render_resource::{RenderResourceId, RenderResourceAssignments},
    renderer::RenderContext,
};
use bevy_asset::Handle;
use std::ops::Range;

pub trait RenderPass {
    fn get_render_context(&self) -> &dyn RenderContext;
    fn set_index_buffer(&mut self, resource: RenderResourceId, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, resource: RenderResourceId, offset: u64);
    fn set_pipeline(&mut self, pipeline_handle: Handle<PipelineDescriptor>);
    fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32);
    fn set_stencil_reference(&mut self, reference: u32);
    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
    // TODO: try to somehow take into account the "set" pipeline instead of passing it in here
    fn set_render_resources(
        &mut self,
        pipeline_descriptor: &PipelineDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<Range<u32>>;
}
