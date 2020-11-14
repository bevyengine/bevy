use crate::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    renderer::{BindGroupId, BufferId, RenderContext},
};
use bevy_asset::Handle;
use std::ops::Range;

pub trait RenderPass {
    fn get_render_context(&self) -> &dyn RenderContext;
    fn set_index_buffer(&mut self, buffer: BufferId, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, buffer: BufferId, offset: u64);
    fn set_pipeline(&mut self, pipeline_handle: &Handle<PipelineDescriptor>);
    fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32);
    fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32);
    fn set_stencil_reference(&mut self, reference: u32);
    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<&[u32]>,
    );
}
