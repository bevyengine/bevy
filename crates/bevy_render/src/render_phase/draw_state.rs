use crate::{
    camera::Viewport,
    diagnostic::internal::{Pass, PassKind, WritePipelineStatistics, WriteTimestamp},
    frame_graph::{
        render_pass_builder::RenderPassBuilder, BindGroupHandle, FrameGraphBuffer,
        FrameGraphTexture, ResourceRead, ResourceRef,
    },
    render_resource::{
        BindGroup, BindGroupId, BindGroupLayoutId, Buffer, BufferId, BufferSlice,
        CachedRenderPipelineId, ShaderStages, Texture,
    },
    renderer::RenderDevice,
};
use bevy_color::LinearRgba;
use bevy_utils::default;
use core::ops::Range;
use std::ops::RangeBounds;
use wgpu::{IndexFormat, QuerySet};

#[cfg(feature = "detailed_trace")]
use tracing::trace;

#[derive(Debug, Default)]
struct DrawState {
    pipeline: Option<CachedRenderPipelineId>,
    bind_groups: Vec<(Option<BindGroupId>, Vec<u32>)>,
    bind_group_layouts: Vec<(Option<BindGroupLayoutId>, Vec<u32>)>,
    /// List of vertex buffers by [`BufferId`], offset, and size. See [`DrawState::buffer_slice_key`]
    vertex_buffers: Vec<Option<(BufferId, u64, u64)>>,
    index_buffer: Option<(BufferId, u64, IndexFormat)>,

    /// Stores whether this state is populated or empty for quick state invalidation
    stores_state: bool,
}

impl DrawState {
    /// Marks the `pipeline` as bound.
    fn set_pipeline(&mut self, pipeline: CachedRenderPipelineId) {
        // TODO: do these need to be cleared?
        // self.bind_groups.clear();
        // self.vertex_buffers.clear();
        // self.index_buffer = None;
        self.pipeline = Some(pipeline);
        self.stores_state = true;
    }

    /// Checks, whether the `pipeline` is already bound.
    fn is_pipeline_set(&self, pipeline: CachedRenderPipelineId) -> bool {
        self.pipeline == Some(pipeline)
    }

    fn set_bind_group_layout(
        &mut self,
        index: usize,
        bind_group_layout: BindGroupLayoutId,
        dynamic_indices: &[u32],
    ) {
        let group = &mut self.bind_group_layouts[index];
        group.0 = Some(bind_group_layout);
        group.1.clear();
        group.1.extend(dynamic_indices);
        self.stores_state = true;
    }

    /// Marks the `bind_group` as bound to the `index`.
    fn set_bind_group(&mut self, index: usize, bind_group: BindGroupId, dynamic_indices: &[u32]) {
        let group = &mut self.bind_groups[index];
        group.0 = Some(bind_group);
        group.1.clear();
        group.1.extend(dynamic_indices);
        self.stores_state = true;
    }

    /// Checks, whether the `bind_group` is already bound to the `index`.
    fn is_bind_group_set(
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

    fn is_bind_group_layout_set(
        &self,
        index: usize,
        bind_group_layout: BindGroupLayoutId,
        dynamic_indices: &[u32],
    ) -> bool {
        if let Some(current_bind_group) = self.bind_group_layouts.get(index) {
            current_bind_group.0 == Some(bind_group_layout)
                && dynamic_indices == current_bind_group.1
        } else {
            false
        }
    }

    /// Marks the vertex `buffer` as bound to the `index`.
    fn set_vertex_buffer(&mut self, index: usize, buffer_slice: &BufferSlice) {
        self.vertex_buffers[index] = Some(self.buffer_slice_key(buffer_slice));
        self.stores_state = true;
    }

    /// Checks, whether the vertex `buffer` is already bound to the `index`.
    fn is_vertex_buffer_set(&self, index: usize, buffer_slice: &BufferSlice) -> bool {
        if let Some(current) = self.vertex_buffers.get(index) {
            *current == Some(self.buffer_slice_key(buffer_slice))
        } else {
            false
        }
    }

    /// Returns the value used for checking whether `BufferSlice`s are equivalent.
    fn buffer_slice_key(&self, buffer_slice: &BufferSlice) -> (BufferId, u64, u64) {
        (
            buffer_slice.id(),
            buffer_slice.offset(),
            buffer_slice.size(),
        )
    }

    /// Marks the index `buffer` as bound.
    fn set_index_buffer(&mut self, buffer: BufferId, offset: u64, index_format: IndexFormat) {
        self.index_buffer = Some((buffer, offset, index_format));
        self.stores_state = true;
    }

    /// Checks, whether the index `buffer` is already bound.
    fn is_index_buffer_set(
        &self,
        buffer: BufferId,
        offset: u64,
        index_format: IndexFormat,
    ) -> bool {
        self.index_buffer == Some((buffer, offset, index_format))
    }
}

/// A [`RenderPass`], which tracks the current pipeline state to skip redundant operations.
///
/// It is used to set the current [`RenderPipeline`], [`BindGroup`]s and [`Buffer`]s.
/// After all requirements are specified, draw calls can be issued.
pub struct TrackedRenderPass<'a> {
    pass: RenderPassBuilder<'a>,
    state: DrawState,
}

impl<'a> TrackedRenderPass<'a> {
    pub fn import_and_read_buffer(
        &mut self,
        buffer: &Buffer,
    ) -> ResourceRef<FrameGraphBuffer, ResourceRead> {
        self.pass.import_and_read_buffer(buffer)
    }

    pub fn import_and_read_texture(
        &mut self,
        texture: &Texture,
    ) -> ResourceRef<FrameGraphTexture, ResourceRead> {
        self.pass.import_and_read_texture(texture)
    }

    /// Tracks the supplied render pass.
    pub fn new(device: &RenderDevice, pass: RenderPassBuilder<'a>) -> Self {
        let limits = device.limits();
        let max_bind_groups = limits.max_bind_groups as usize;
        let max_vertex_buffers = limits.max_vertex_buffers as usize;
        Self {
            state: DrawState {
                bind_groups: vec![(None, Vec::new()); max_bind_groups],
                bind_group_layouts: vec![(None, Vec::new()); max_bind_groups],
                vertex_buffers: vec![None; max_vertex_buffers],
                ..default()
            },
            pass,
        }
    }

    /// Sets the active [`RenderPipeline`].
    ///
    /// Subsequent draw calls will exhibit the behavior defined by the `pipeline`.
    pub fn set_render_pipeline(&mut self, pipeline: CachedRenderPipelineId) {
        if self.state.is_pipeline_set(pipeline) {
            return;
        }
        self.state.set_pipeline(pipeline);
        self.pass.set_render_pipeline(pipeline);
    }

    pub fn set_bind_group_handle(
        &mut self,
        index: usize,
        bind_group: &'a BindGroupHandle,
        dynamic_uniform_indices: &[u32],
    ) {
        if self.state.is_bind_group_layout_set(
            index,
            bind_group.layout.id(),
            dynamic_uniform_indices,
        ) {
            #[cfg(feature = "detailed_trace")]
            trace!(
                "set bind_group {} (already set): {:?} ({:?})",
                index,
                bind_group,
                dynamic_uniform_indices
            );
            return;
        }

        self.state
            .set_bind_group_layout(index, bind_group.layout.id(), dynamic_uniform_indices);
        self.pass
            .set_bind_group(index as u32, bind_group.clone(), dynamic_uniform_indices);
    }

    /// Sets the active bind group for a given bind group index. The bind group layout
    /// in the active pipeline when any `draw()` function is called must match the layout of
    /// this bind group.
    ///
    /// If the bind group have dynamic offsets, provide them in binding order.
    /// These offsets have to be aligned to [`WgpuLimits::min_uniform_buffer_offset_alignment`](crate::settings::WgpuLimits::min_uniform_buffer_offset_alignment)
    /// or [`WgpuLimits::min_storage_buffer_offset_alignment`](crate::settings::WgpuLimits::min_storage_buffer_offset_alignment) appropriately.
    pub fn set_bind_group(
        &mut self,
        index: usize,
        bind_group: &'a BindGroup,
        dynamic_uniform_indices: &[u32],
    ) {
        if self
            .state
            .is_bind_group_set(index, bind_group.id(), dynamic_uniform_indices)
        {
            #[cfg(feature = "detailed_trace")]
            trace!(
                "set bind_group {} (already set): {:?} ({:?})",
                index,
                bind_group,
                dynamic_uniform_indices
            );
            return;
        }

        self.state
            .set_bind_group(index, bind_group.id(), dynamic_uniform_indices);
        self.pass
            .set_raw_bind_group(index as u32, Some(bind_group), dynamic_uniform_indices);
    }

    /// Assign a vertex buffer to a slot.
    ///
    /// Subsequent calls to [`draw`] and [`draw_indexed`] on this
    /// [`TrackedRenderPass`] will use `buffer` as one of the source vertex buffers.
    ///
    /// The `slot_index` refers to the index of the matching descriptor in
    /// [`VertexState::buffers`](crate::render_resource::VertexState::buffers).
    ///
    /// [`draw`]: TrackedRenderPass::draw
    /// [`draw_indexed`]: TrackedRenderPass::draw_indexed
    pub fn set_vertex_buffer(
        &mut self,
        slot_index: usize,
        buffer: &Buffer,
        bounds: impl RangeBounds<wgpu::BufferAddress>,
    ) {
        let vertex_read = self.pass.import_and_read_buffer(buffer);

        let buffer_slice = buffer.slice(bounds);

        if self.state.is_vertex_buffer_set(slot_index, &buffer_slice) {
            #[cfg(feature = "detailed_trace")]
            trace!(
                "set vertex buffer {} (already set): {:?} (offset = {}, size = {})",
                slot_index,
                buffer_slice.id(),
                buffer_slice.offset(),
                buffer_slice.size(),
            );
            return;
        }
        #[cfg(feature = "detailed_trace")]
        trace!(
            "set vertex buffer {}: {:?} (offset = {}, size = {})",
            slot_index,
            buffer_slice.id(),
            buffer_slice.offset(),
            buffer_slice.size(),
        );

        self.state.set_vertex_buffer(slot_index, &buffer_slice);
        self.pass.set_vertex_buffer(
            slot_index as u32,
            &vertex_read,
            buffer_slice.offset(),
            buffer_slice.size(),
        );
    }

    /// Sets the active index buffer.
    ///
    /// Subsequent calls to [`TrackedRenderPass::draw_indexed`] will use the buffer referenced by
    /// `buffer_slice` as the source index buffer.
    pub fn set_index_buffer(
        &mut self,
        buffer: &Buffer,
        bounds: impl RangeBounds<wgpu::BufferAddress>,
        offset: u64,
        index_format: IndexFormat,
    ) {
        let index_read = self.pass.import_and_read_buffer(buffer);
        let buffer_slice = buffer.slice(bounds);

        if self
            .state
            .is_index_buffer_set(buffer_slice.id(), offset, index_format)
        {
            #[cfg(feature = "detailed_trace")]
            trace!(
                "set index buffer (already set): {:?} ({})",
                buffer_slice.id(),
                offset
            );
            return;
        }
        #[cfg(feature = "detailed_trace")]
        trace!("set index buffer: {:?} ({})", buffer_slice.id(), offset);

        self.state
            .set_index_buffer(buffer.id(), offset, index_format);
        self.pass.set_index_buffer(
            &index_read,
            index_format,
            buffer_slice.offset(),
            buffer_slice.size(),
        );
    }

    /// Draws primitives from the active vertex buffer(s).
    ///
    /// The active vertex buffer(s) can be set with [`TrackedRenderPass::set_vertex_buffer`].
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        #[cfg(feature = "detailed_trace")]
        trace!("draw: {:?} {:?}", vertices, instances);
        self.pass.draw(vertices, instances);
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffer(s).
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffer(s) can be set with [`TrackedRenderPass::set_vertex_buffer`].
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "draw indexed: {:?} {} {:?}",
            indices,
            base_vertex,
            instances
        );
        self.pass.draw_indexed(indices, base_vertex, instances);
    }

    /// Draws primitives from the active vertex buffer(s) based on the contents of the
    /// `indirect_buffer`.
    ///
    /// The active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// The structure expected in `indirect_buffer` is the following:
    ///
    /// ```
    /// #[repr(C)]
    /// struct DrawIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_vertex: u32, // The Index of the first vertex to draw.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn draw_indirect(&mut self, indirect_buffer: &'a Buffer, indirect_offset: u64) {
        #[cfg(feature = "detailed_trace")]
        trace!("draw indirect: {:?} {}", indirect_buffer, indirect_offset);

        let indirect_buffer_read = self.pass.import_and_read_buffer(indirect_buffer);

        self.pass
            .draw_indirect(&indirect_buffer_read, indirect_offset);
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffers,
    /// based on the contents of the `indirect_buffer`.
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// The structure expected in `indirect_buffer` is the following:
    ///
    /// ```
    /// #[repr(C)]
    /// struct DrawIndexedIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_index: u32, // The base index within the index buffer.
    ///     vertex_offset: i32, // The value added to the vertex index before indexing into the vertex buffer.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn draw_indexed_indirect(&mut self, indirect_buffer: &'a Buffer, indirect_offset: u64) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "draw indexed indirect: {:?} {}",
            indirect_buffer,
            indirect_offset
        );

        let indirect_buffer_read = self.pass.import_and_read_buffer(indirect_buffer);

        self.pass
            .draw_indexed_indirect(&indirect_buffer_read, indirect_offset);
    }

    /// Dispatches multiple draw calls from the active vertex buffer(s) based on the contents of the
    /// `indirect_buffer`.`count` draw calls are issued.
    ///
    /// The active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// `indirect_buffer` should contain `count` tightly packed elements of the following structure:
    ///
    /// ```
    /// #[repr(C)]
    /// struct DrawIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_vertex: u32, // The Index of the first vertex to draw.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn multi_draw_indirect(
        &mut self,
        indirect_buffer: &'a Buffer,
        indirect_offset: u64,
        count: u32,
    ) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "multi draw indirect: {:?} {}, {}x",
            indirect_buffer,
            indirect_offset,
            count
        );

        let indirect_buffer_read = self.pass.import_and_read_buffer(indirect_buffer);

        self.pass
            .multi_draw_indirect(&indirect_buffer_read, indirect_offset, count);
    }

    /// Dispatches multiple draw calls from the active vertex buffer(s) based on the contents of
    /// the `indirect_buffer`.
    /// The count buffer is read to determine how many draws to issue.
    ///
    /// The indirect buffer must be long enough to account for `max_count` draws, however only
    /// `count` elements will be read, where `count` is the value read from `count_buffer` capped
    /// at `max_count`.
    ///
    /// The active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// `indirect_buffer` should contain `count` tightly packed elements of the following structure:
    ///
    /// ```
    /// #[repr(C)]
    /// struct DrawIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_vertex: u32, // The Index of the first vertex to draw.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn multi_draw_indirect_count(
        &mut self,
        indirect_buffer: &'a Buffer,
        indirect_offset: u64,
        count_buffer: &'a Buffer,
        count_offset: u64,
        max_count: u32,
    ) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "multi draw indirect count: {:?} {}, ({:?} {})x, max {}x",
            indirect_buffer,
            indirect_offset,
            count_buffer,
            count_offset,
            max_count
        );

        let indirect_buffer_read = self.pass.import_and_read_buffer(indirect_buffer);
        let count_buffer_read = self.pass.import_and_read_buffer(count_buffer);

        self.pass.multi_draw_indirect_count(
            &indirect_buffer_read,
            indirect_offset,
            &count_buffer_read,
            count_offset,
            max_count,
        );
    }

    /// Dispatches multiple draw calls from the active index buffer and the active vertex buffers,
    /// based on the contents of the `indirect_buffer`. `count` draw calls are issued.
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// `indirect_buffer` should contain `count` tightly packed elements of the following structure:
    ///
    /// ```
    /// #[repr(C)]
    /// struct DrawIndexedIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_index: u32, // The base index within the index buffer.
    ///     vertex_offset: i32, // The value added to the vertex index before indexing into the vertex buffer.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn multi_draw_indexed_indirect(
        &mut self,
        indirect_buffer: &'a Buffer,
        indirect_offset: u64,
        count: u32,
    ) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "multi draw indexed indirect: {:?} {}, {}x",
            indirect_buffer,
            indirect_offset,
            count
        );
        let indirect_buffer_read = self.pass.import_and_read_buffer(indirect_buffer);

        self.pass
            .multi_draw_indexed_indirect(&indirect_buffer_read, indirect_offset, count);
    }

    /// Dispatches multiple draw calls from the active index buffer and the active vertex buffers,
    /// based on the contents of the `indirect_buffer`.
    /// The count buffer is read to determine how many draws to issue.
    ///
    /// The indirect buffer must be long enough to account for `max_count` draws, however only
    /// `count` elements will be read, where `count` is the value read from `count_buffer` capped
    /// at `max_count`.
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// `indirect_buffer` should contain `count` tightly packed elements of the following structure:
    ///
    /// ```
    /// #[repr(C)]
    /// struct DrawIndexedIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_index: u32, // The base index within the index buffer.
    ///     vertex_offset: i32, // The value added to the vertex index before indexing into the vertex buffer.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn multi_draw_indexed_indirect_count(
        &mut self,
        indirect_buffer: &'a Buffer,
        indirect_offset: u64,
        count_buffer: &'a Buffer,
        count_offset: u64,
        max_count: u32,
    ) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "multi draw indexed indirect count: {:?} {}, ({:?} {})x, max {}x",
            indirect_buffer,
            indirect_offset,
            count_buffer,
            count_offset,
            max_count
        );

        let indirect_buffer_read = self.pass.import_and_read_buffer(indirect_buffer);
        let count_buffer_read = self.pass.import_and_read_buffer(count_buffer);

        self.pass.multi_draw_indexed_indirect_count(
            &indirect_buffer_read,
            indirect_offset,
            &count_buffer_read,
            count_offset,
            max_count,
        );
    }

    /// Sets the stencil reference.
    ///
    /// Subsequent stencil tests will test against this value.
    pub fn set_stencil_reference(&mut self, reference: u32) {
        #[cfg(feature = "detailed_trace")]
        trace!("set stencil reference: {}", reference);
        self.pass.set_stencil_reference(reference);
    }

    /// Sets the scissor region.
    ///
    /// Subsequent draw calls will discard any fragments that fall outside this region.
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        #[cfg(feature = "detailed_trace")]
        trace!("set_scissor_rect: {} {} {} {}", x, y, width, height);
        self.pass.set_scissor_rect(x, y, width, height);
    }

    /// Set push constant data.
    ///
    /// `Features::PUSH_CONSTANTS` must be enabled on the device in order to call these functions.
    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "set push constants: {:?} offset: {} data.len: {}",
            stages,
            offset,
            data.len()
        );
        self.pass.set_push_constants(stages, offset, data);
    }

    /// Set the rendering viewport.
    ///
    /// Subsequent draw calls will be projected into that viewport.
    pub fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        #[cfg(feature = "detailed_trace")]
        trace!(
            "set viewport: {} {} {} {} {} {}",
            x,
            y,
            width,
            height,
            min_depth,
            max_depth
        );
        self.pass
            .set_viewport(x, y, width, height, min_depth, max_depth);
    }

    /// Set the rendering viewport to the given camera [`Viewport`].
    ///
    /// Subsequent draw calls will be projected into that viewport.
    pub fn set_camera_viewport(&mut self, viewport: &Viewport) {
        self.set_viewport(
            viewport.physical_position.x as f32,
            viewport.physical_position.y as f32,
            viewport.physical_size.x as f32,
            viewport.physical_size.y as f32,
            viewport.depth.start,
            viewport.depth.end,
        );
    }

    /// Insert a single debug marker.
    ///
    /// This is a GPU debugging feature. This has no effect on the rendering itself.
    pub fn insert_debug_marker(&mut self, label: &str) {
        #[cfg(feature = "detailed_trace")]
        trace!("insert debug marker: {}", label);
        self.pass.insert_debug_marker(label);
    }

    /// Start a new debug group.
    ///
    /// Push a new debug group over the internal stack. Subsequent render commands and debug
    /// markers are grouped into this new group, until [`pop_debug_group`] is called.
    ///
    /// ```
    /// # fn example(mut pass: bevy_render::render_phase::TrackedRenderPass<'static>) {
    /// pass.push_debug_group("Render the car");
    /// // [setup pipeline etc...]
    /// pass.draw(0..64, 0..1);
    /// pass.pop_debug_group();
    /// # }
    /// ```
    ///
    /// Note that [`push_debug_group`] and [`pop_debug_group`] must always be called in pairs.
    ///
    /// This is a GPU debugging feature. This has no effect on the rendering itself.
    ///
    /// [`push_debug_group`]: TrackedRenderPass::push_debug_group
    /// [`pop_debug_group`]: TrackedRenderPass::pop_debug_group
    pub fn push_debug_group(&mut self, label: &str) {
        #[cfg(feature = "detailed_trace")]
        trace!("push_debug_group marker: {}", label);
        self.pass.push_debug_group(label);
    }

    /// End the current debug group.
    ///
    /// Subsequent render commands and debug markers are not grouped anymore in
    /// this group, but in the previous one (if any) or the default top-level one
    /// if the debug group was the last one on the stack.
    ///
    /// Note that [`push_debug_group`] and [`pop_debug_group`] must always be called in pairs.
    ///
    /// This is a GPU debugging feature. This has no effect on the rendering itself.
    ///
    /// [`push_debug_group`]: TrackedRenderPass::push_debug_group
    /// [`pop_debug_group`]: TrackedRenderPass::pop_debug_group
    pub fn pop_debug_group(&mut self) {
        #[cfg(feature = "detailed_trace")]
        trace!("pop_debug_group");
        self.pass.pop_debug_group();
    }

    /// Sets the blend color as used by some of the blending modes.
    ///
    /// Subsequent blending tests will test against this value.
    pub fn set_blend_constant(&mut self, color: LinearRgba) {
        #[cfg(feature = "detailed_trace")]
        trace!("set blend constant: {:?}", color);
        self.pass.set_blend_constant(color);
    }
}

impl WriteTimestamp for TrackedRenderPass<'_> {
    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        self.pass.write_timestamp(query_set, index);
    }
}

impl WritePipelineStatistics for TrackedRenderPass<'_> {
    fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        self.pass.begin_pipeline_statistics_query(query_set, index);
    }

    fn end_pipeline_statistics_query(&mut self) {
        self.pass.end_pipeline_statistics_query();
    }
}

impl Pass for TrackedRenderPass<'_> {
    const KIND: PassKind = PassKind::Render;
}
