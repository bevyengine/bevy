use crate::{
    prelude::Color,
    render_resource::{
        BindGroup, BindGroupId, BufferId, BufferSlice, RenderPipeline, RenderPipelineId,
        ShaderStages,
    },
};
use bevy_utils::tracing::debug;
use std::ops::Range;
use wgpu::{IndexFormat, RenderPass};

/// Tracks the current [`TrackedRenderPass`] state to ensure draw calls are valid.
#[derive(Debug, Default)]
pub struct DrawState {
    pipeline: Option<RenderPipelineId>,
    bind_groups: Vec<(Option<BindGroupId>, Vec<u32>)>,
    vertex_buffers: Vec<Option<(BufferId, u64)>>,
    index_buffer: Option<(BufferId, u64, IndexFormat)>,
}

impl DrawState {
    pub fn set_bind_group(
        &mut self,
        index: usize,
        bind_group: BindGroupId,
        dynamic_indices: &[u32],
    ) {
        if index >= self.bind_groups.len() {
            self.bind_groups.resize(index + 1, (None, Vec::new()));
        }
        self.bind_groups[index].0 = Some(bind_group);
        self.bind_groups[index].1.clear();
        self.bind_groups[index].1.extend(dynamic_indices);
    }

    pub fn is_bind_group_set(
        &self,
        index: usize,
        bind_group: BindGroupId,
        dynamic_indices: &[u32],
    ) -> bool {
        if let Some(current_bind_group) = self.bind_groups.get(index) {
            current_bind_group.0 == Some(bind_group) && dynamic_indices == current_bind_group.1
        } else {
            false
        }
    }

    pub fn set_vertex_buffer(&mut self, index: usize, buffer: BufferId, offset: u64) {
        if index >= self.vertex_buffers.len() {
            self.vertex_buffers.resize(index + 1, None);
        }
        self.vertex_buffers[index] = Some((buffer, offset));
    }

    pub fn is_vertex_buffer_set(&self, index: usize, buffer: BufferId, offset: u64) -> bool {
        if let Some(current) = self.vertex_buffers.get(index) {
            *current == Some((buffer, offset))
        } else {
            false
        }
    }

    pub fn set_index_buffer(&mut self, buffer: BufferId, offset: u64, index_format: IndexFormat) {
        self.index_buffer = Some((buffer, offset, index_format));
    }

    pub fn is_index_buffer_set(
        &self,
        buffer: BufferId,
        offset: u64,
        index_format: IndexFormat,
    ) -> bool {
        self.index_buffer == Some((buffer, offset, index_format))
    }

    pub fn is_pipeline_set(&self, pipeline: RenderPipelineId) -> bool {
        self.pipeline == Some(pipeline)
    }

    pub fn set_pipeline(&mut self, pipeline: RenderPipelineId) {
        // TODO: do these need to be cleared?
        // self.bind_groups.clear();
        // self.vertex_buffers.clear();
        // self.index_buffer = None;
        self.pipeline = Some(pipeline);
    }
}

/// A [`RenderPass`], which tracks the current pipeline state to ensure all draw calls are valid.
/// It is used to set the current [`RenderPipeline`], [`BindGroups`](BindGroup) and buffers.
/// After all requirements are specified, draw calls can be issued.
pub struct TrackedRenderPass<'a> {
    pass: RenderPass<'a>,
    state: DrawState,
}

impl<'a> TrackedRenderPass<'a> {
    /// Tracks the supplied render pass.
    pub fn new(pass: RenderPass<'a>) -> Self {
        Self {
            state: DrawState::default(),
            pass,
        }
    }

    /// Sets the active [`RenderPipeline`].
    ///
    /// Subsequent draw calls will exhibit the behavior defined by the `pipeline`.
    pub fn set_render_pipeline(&mut self, pipeline: &'a RenderPipeline) {
        debug!("set pipeline: {:?}", pipeline);
        if self.state.is_pipeline_set(pipeline.id()) {
            return;
        }
        self.pass.set_pipeline(pipeline);
        self.state.set_pipeline(pipeline.id());
    }

    /// Sets the active [`BindGroup`] for a given bind group index. The bind group layout in the
    /// active pipeline when any `draw()` function is called must match the layout
    /// of this `bind group`.
    pub fn set_bind_group(
        &mut self,
        index: usize,
        bind_group: &'a BindGroup,
        dynamic_uniform_indices: &[u32],
    ) {
        if self
            .state
            .is_bind_group_set(index as usize, bind_group.id(), dynamic_uniform_indices)
        {
            debug!(
                "set bind_group {} (already set): {:?} ({:?})",
                index, bind_group, dynamic_uniform_indices
            );
            return;
        } else {
            debug!(
                "set bind_group {}: {:?} ({:?})",
                index, bind_group, dynamic_uniform_indices
            );
        }
        self.pass
            .set_bind_group(index as u32, bind_group, dynamic_uniform_indices);
        self.state
            .set_bind_group(index as usize, bind_group.id(), dynamic_uniform_indices);
    }

    /// Assign a vertex buffer to a slot.
    ///
    /// Subsequent calls to [`TrackedRenderPass::draw`] and [`TrackedRenderPass::draw_indexed`]
    /// will use the `buffer` as one of the source vertex buffers.
    ///
    /// The `slot` refers to the index of the matching descriptor in
    /// [`VertexState::buffers`](crate::render_resource::VertexState::buffers).
    pub fn set_vertex_buffer(&mut self, index: usize, buffer_slice: BufferSlice<'a>) {
        let offset = buffer_slice.offset();
        if self
            .state
            .is_vertex_buffer_set(index, buffer_slice.id(), offset)
        {
            debug!(
                "set vertex buffer {} (already set): {:?} ({})",
                index,
                buffer_slice.id(),
                offset
            );
            return;
        } else {
            debug!(
                "set vertex buffer {}: {:?} ({})",
                index,
                buffer_slice.id(),
                offset
            );
        }
        self.pass.set_vertex_buffer(index as u32, *buffer_slice);
        self.state
            .set_vertex_buffer(index, buffer_slice.id(), offset);
    }

    /// Sets the active index buffer.
    ///
    /// Subsequent calls to [`TrackedRenderPass::draw_indexed`] will use the `buffer` as
    /// the source index buffer.
    pub fn set_index_buffer(
        &mut self,
        buffer_slice: BufferSlice<'a>,
        offset: u64,
        index_format: IndexFormat,
    ) {
        if self
            .state
            .is_index_buffer_set(buffer_slice.id(), offset, index_format)
        {
            debug!(
                "set index buffer (already set): {:?} ({})",
                buffer_slice.id(),
                offset
            );
            return;
        } else {
            debug!("set index buffer: {:?} ({})", buffer_slice.id(), offset);
        }
        self.pass.set_index_buffer(*buffer_slice, index_format);
        self.state
            .set_index_buffer(buffer_slice.id(), offset, index_format);
    }

    /// Draws primitives from the active vertex buffer(s).
    ///
    /// The active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        debug!("draw: {:?} {:?}", vertices, instances);
        self.pass.draw(vertices, instances);
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffer(s).
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        debug!(
            "draw indexed: {:?} {} {:?}",
            indices, base_vertex, instances
        );
        self.pass.draw_indexed(indices, base_vertex, instances);
    }

    pub fn set_stencil_reference(&mut self, reference: u32) {
        debug!("set stencil reference: {}", reference);

        self.pass.set_stencil_reference(reference);
    }

    /// Sets the scissor region.
    /// Subsequent draw calls will discard any fragments that fall outside this region.
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        debug!("set_scissor_rect: {} {} {} {}", x, y, width, height);
        self.pass.set_scissor_rect(x, y, width, height);
    }

    /// Set push constant data.
    ///
    /// Features::PUSH_CONSTANTS must be enabled on the device in order to call these functions.
    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        debug!(
            "set push constants: {:?} offset: {} data.len: {}",
            stages,
            offset,
            data.len()
        );
        self.pass.set_push_constants(stages, offset, data)
    }

    pub fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        debug!(
            "set viewport: {} {} {} {} {} {}",
            x, y, width, height, min_depth, max_depth
        );
        self.pass
            .set_viewport(x, y, width, height, min_depth, max_depth)
    }

    pub fn insert_debug_marker(&mut self, label: &str) {
        debug!("insert debug marker: {}", label);
        self.pass.insert_debug_marker(label)
    }

    pub fn push_debug_group(&mut self, label: &str) {
        debug!("push_debug_group marker: {}", label);
        self.pass.push_debug_group(label)
    }

    pub fn pop_debug_group(&mut self) {
        debug!("pop_debug_group");
        self.pass.pop_debug_group()
    }

    pub fn set_blend_constant(&mut self, color: Color) {
        debug!("set blend constant: {:?}", color);
        self.pass.set_blend_constant(wgpu::Color::from(color))
    }
}
