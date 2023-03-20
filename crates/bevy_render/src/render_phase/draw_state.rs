use crate::{
    camera::ExtractedCamera,
    prelude::Color,
    render_phase::{DrawFunctions, PhaseItem, RenderPhase},
    render_resource::{
        BindGroup, BindGroupId, Buffer, BufferId, BufferSlice, RenderPipeline, RenderPipelineId,
        ShaderStages, TextureView,
    },
    renderer::{RenderContext, RenderDevice},
    view::ViewTarget,
};
use bevy_ecs::prelude::*;
use bevy_utils::{default, detailed_trace};
use std::ops::Range;
use wgpu::{
    IndexFormat, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor,
};
use wgpu_hal::{MAX_BIND_GROUPS, MAX_VERTEX_BUFFERS};

/// Tracks the state of a [`TrackedRenderPass`].
///
/// This is used to skip redundant operations on the [`TrackedRenderPass`] (e.g. setting an already
/// set pipeline, binding an already bound bind group). These operations can otherwise be fairly
/// costly due to IO to the GPU, so deduplicating these calls results in a speedup.
#[derive(Debug, Default)]
struct DrawState {
    pipeline: Option<RenderPipelineId>,
    bind_groups: Vec<(Option<BindGroupId>, Vec<u32>)>,
    vertex_buffers: Vec<Option<(BufferId, u64)>>,
    index_buffer: Option<(BufferId, u64, IndexFormat)>,
}

impl DrawState {
    /// Marks the `pipeline` as bound.
    pub fn set_pipeline(&mut self, pipeline: RenderPipelineId) {
        // TODO: do these need to be cleared?
        // self.bind_groups.clear();
        // self.vertex_buffers.clear();
        // self.index_buffer = None;
        self.pipeline = Some(pipeline);
    }

    /// Checks, whether the `pipeline` is already bound.
    pub fn is_pipeline_set(&self, pipeline: RenderPipelineId) -> bool {
        self.pipeline == Some(pipeline)
    }

    /// Marks the `bind_group` as bound to the `index`.
    pub fn set_bind_group(
        &mut self,
        index: usize,
        bind_group: BindGroupId,
        dynamic_indices: &[u32],
    ) {
        let group = &mut self.bind_groups[index];
        group.0 = Some(bind_group);
        group.1.clear();
        group.1.extend(dynamic_indices);
    }

    /// Checks, whether the `bind_group` is already bound to the `index`.
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

    /// Marks the vertex `buffer` as bound to the `index`.
    pub fn set_vertex_buffer(&mut self, index: usize, buffer: BufferId, offset: u64) {
        self.vertex_buffers[index] = Some((buffer, offset));
    }

    /// Checks, whether the vertex `buffer` is already bound to the `index`.
    pub fn is_vertex_buffer_set(&self, index: usize, buffer: BufferId, offset: u64) -> bool {
        if let Some(current) = self.vertex_buffers.get(index) {
            *current == Some((buffer, offset))
        } else {
            false
        }
    }

    /// Marks the index `buffer` as bound.
    pub fn set_index_buffer(&mut self, buffer: BufferId, offset: u64, index_format: IndexFormat) {
        self.index_buffer = Some((buffer, offset, index_format));
    }

    /// Checks, whether the index `buffer` is already bound.
    pub fn is_index_buffer_set(
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
    pass: RenderPass<'a>,
    state: DrawState,
    view_entity: Entity,
}

impl<'a> TrackedRenderPass<'a> {
    pub(crate) fn new(device: &RenderDevice, pass: RenderPass<'a>, view_entity: Entity) -> Self {
        let limits = device.limits();
        let max_bind_groups = limits.max_bind_groups as usize;
        let max_vertex_buffers = limits.max_vertex_buffers as usize;
        Self {
            state: DrawState {
                bind_groups: vec![(None, Vec::new()); max_bind_groups.min(MAX_BIND_GROUPS)],
                vertex_buffers: vec![None; max_vertex_buffers.min(MAX_VERTEX_BUFFERS)],
                ..default()
            },
            pass,
            view_entity,
        }
    }

    /// Sets the active [`RenderPipeline`].
    ///
    /// Subsequent draw calls will exhibit the behavior defined by this `pipeline`.
    pub fn set_pipeline(&mut self, pipeline: &'a RenderPipeline) -> &mut Self {
        detailed_trace!("set pipeline: {:?}", pipeline);
        if self.state.is_pipeline_set(pipeline.id()) {
            return self;
        }
        self.pass.set_pipeline(pipeline);
        self.state.set_pipeline(pipeline.id());
        self
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
    ) -> &mut Self {
        if self
            .state
            .is_bind_group_set(index, bind_group.id(), dynamic_uniform_indices)
        {
            detailed_trace!(
                "set bind_group {} (already set): {:?} ({:?})",
                index,
                bind_group,
                dynamic_uniform_indices
            );
            return self;
        }
        detailed_trace!(
            "set bind_group {}: {:?} ({:?})",
            index,
            bind_group,
            dynamic_uniform_indices
        );

        self.pass
            .set_bind_group(index as u32, bind_group, dynamic_uniform_indices);
        self.state
            .set_bind_group(index, bind_group.id(), dynamic_uniform_indices);
        self
    }

    /// Assign a vertex buffer to a slot.
    ///
    /// Subsequent calls to [`draw`] and [`draw_indexed`] on this
    /// [`RenderPass`] will use `buffer` as one of the source vertex buffers.
    ///
    /// The `slot_index` refers to the index of the matching descriptor in
    /// [`VertexState::buffers`](crate::render_resource::VertexState::buffers).
    ///
    /// [`draw`]: TrackedRenderPass::draw
    /// [`draw_indexed`]: TrackedRenderPass::draw_indexed
    pub fn set_vertex_buffer(
        &mut self,
        slot_index: usize,
        buffer_slice: BufferSlice<'a>,
    ) -> &mut Self {
        let offset = buffer_slice.offset();
        if self
            .state
            .is_vertex_buffer_set(slot_index, buffer_slice.id(), offset)
        {
            detailed_trace!(
                "set vertex buffer {} (already set): {:?} ({})",
                slot_index,
                buffer_slice.id(),
                offset
            );
            return self;
        }
        detailed_trace!(
            "set vertex buffer {}: {:?} ({})",
            slot_index,
            buffer_slice.id(),
            offset
        );

        self.pass
            .set_vertex_buffer(slot_index as u32, *buffer_slice);
        self.state
            .set_vertex_buffer(slot_index, buffer_slice.id(), offset);

        self
    }

    /// Sets the active index buffer.
    ///
    /// Subsequent calls to [`TrackedRenderPass::draw_indexed`] will use the buffer referenced by
    /// `buffer_slice` as the source index buffer.
    pub fn set_index_buffer(
        &mut self,
        buffer_slice: BufferSlice<'a>,
        offset: u64,
        index_format: IndexFormat,
    ) -> &mut Self {
        if self
            .state
            .is_index_buffer_set(buffer_slice.id(), offset, index_format)
        {
            detailed_trace!(
                "set index buffer (already set): {:?} ({})",
                buffer_slice.id(),
                offset
            );
            return self;
        }
        detailed_trace!("set index buffer: {:?} ({})", buffer_slice.id(), offset);
        self.pass.set_index_buffer(*buffer_slice, index_format);
        self.state
            .set_index_buffer(buffer_slice.id(), offset, index_format);
        self
    }

    /// Draws primitives from the active vertex buffer(s).
    ///
    /// The active vertex buffer(s) can be set with [`TrackedRenderPass::set_vertex_buffer`].
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) -> &mut Self {
        detailed_trace!("draw: {:?} {:?}", vertices, instances);
        self.pass.draw(vertices, instances);
        self
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffer(s).
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffer(s) can be set with [`TrackedRenderPass::set_vertex_buffer`].
    pub fn draw_indexed(
        &mut self,
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    ) -> &mut Self {
        detailed_trace!(
            "draw indexed: {:?} {} {:?}",
            indices,
            base_vertex,
            instances
        );
        self.pass.draw_indexed(indices, base_vertex, instances);
        self
    }

    /// Draws primitives from the active vertex buffer(s) based on the contents of the
    /// `indirect_buffer`.
    ///
    /// The active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// The structure expected in `indirect_buffer` is the following:
    ///
    /// ```rust
    /// #[repr(C)]
    /// struct DrawIndirect {
    ///     vertex_count: u32, // The number of vertices to draw.
    ///     instance_count: u32, // The number of instances to draw.
    ///     first_vertex: u32, // The Index of the first vertex to draw.
    ///     first_instance: u32, // The instance ID of the first instance to draw.
    ///     // has to be 0, unless [`Features::INDIRECT_FIRST_INSTANCE`] is enabled.
    /// }
    /// ```
    pub fn draw_indirect(
        &mut self,
        indirect_buffer: &'a Buffer,
        indirect_offset: u64,
    ) -> &mut Self {
        detailed_trace!("draw indirect: {:?} {}", indirect_buffer, indirect_offset);
        self.pass.draw_indirect(indirect_buffer, indirect_offset);
        self
    }

    /// Draws indexed primitives using the active index buffer and the active vertex buffers,
    /// based on the contents of the `indirect_buffer`.
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// The structure expected in `indirect_buffer` is the following:
    ///
    /// ```rust
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
    pub fn draw_indexed_indirect(
        &mut self,
        indirect_buffer: &'a Buffer,
        indirect_offset: u64,
    ) -> &mut Self {
        detailed_trace!(
            "draw indexed indirect: {:?} {}",
            indirect_buffer,
            indirect_offset
        );
        self.pass
            .draw_indexed_indirect(indirect_buffer, indirect_offset);
        self
    }

    /// Dispatches multiple draw calls from the active vertex buffer(s) based on the contents of the
    /// `indirect_buffer`.`count` draw calls are issued.
    ///
    /// The active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// `indirect_buffer` should contain `count` tightly packed elements of the following structure:
    ///
    /// ```rust
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
    ) -> &mut Self {
        detailed_trace!(
            "multi draw indirect: {:?} {}, {}x",
            indirect_buffer,
            indirect_offset,
            count
        );
        self.pass
            .multi_draw_indirect(indirect_buffer, indirect_offset, count);
        self
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
    /// ```rust
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
    ) -> &mut Self {
        detailed_trace!(
            "multi draw indirect count: {:?} {}, ({:?} {})x, max {}x",
            indirect_buffer,
            indirect_offset,
            count_buffer,
            count_offset,
            max_count
        );
        self.pass.multi_draw_indirect_count(
            indirect_buffer,
            indirect_offset,
            count_buffer,
            count_offset,
            max_count,
        );
        self
    }

    /// Dispatches multiple draw calls from the active index buffer and the active vertex buffers,
    /// based on the contents of the `indirect_buffer`. `count` draw calls are issued.
    ///
    /// The active index buffer can be set with [`TrackedRenderPass::set_index_buffer`], while the
    /// active vertex buffers can be set with [`TrackedRenderPass::set_vertex_buffer`].
    ///
    /// `indirect_buffer` should contain `count` tightly packed elements of the following structure:
    ///
    /// ```rust
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
    ) -> &mut Self {
        detailed_trace!(
            "multi draw indexed indirect: {:?} {}, {}x",
            indirect_buffer,
            indirect_offset,
            count
        );
        self.pass
            .multi_draw_indexed_indirect(indirect_buffer, indirect_offset, count);
        self
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
    /// ```rust
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
    ) -> &mut Self {
        detailed_trace!(
            "multi draw indexed indirect count: {:?} {}, ({:?} {})x, max {}x",
            indirect_buffer,
            indirect_offset,
            count_buffer,
            count_offset,
            max_count
        );
        self.pass.multi_draw_indexed_indirect_count(
            indirect_buffer,
            indirect_offset,
            count_buffer,
            count_offset,
            max_count,
        );
        self
    }

    /// Sets the stencil reference.
    ///
    /// Subsequent stencil tests will test against this value.
    pub fn set_stencil_reference(&mut self, reference: u32) -> &mut Self {
        detailed_trace!("set stencil reference: {}", reference);
        self.pass.set_stencil_reference(reference);
        self
    }

    /// Sets the scissor region.
    ///
    /// Subsequent draw calls will discard any fragments that fall outside this region.
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
        detailed_trace!("set_scissor_rect: {} {} {} {}", x, y, width, height);
        self.pass.set_scissor_rect(x, y, width, height);
        self
    }

    /// Set push constant data.
    ///
    /// `Features::PUSH_CONSTANTS` must be enabled on the device in order to call these functions.
    pub fn set_push_constants(
        &mut self,
        stages: ShaderStages,
        offset: u32,
        data: &[u8],
    ) -> &mut Self {
        detailed_trace!(
            "set push constants: {:?} offset: {} data.len: {}",
            stages,
            offset,
            data.len()
        );
        self.pass.set_push_constants(stages, offset, data);
        self
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
    ) -> &mut Self {
        detailed_trace!(
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
        self
    }

    /// Set the rendering viewport to the [`Viewport`](crate::camera::Viewport) of the given [`ExtractedCamera`].
    ///
    /// Subsequent draw calls will be projected into that viewport.
    pub fn set_camera_viewport(&mut self, camera: &ExtractedCamera) -> &mut Self {
        if let Some(viewport) = &camera.viewport {
            self.set_viewport(
                viewport.physical_position.x as f32,
                viewport.physical_position.y as f32,
                viewport.physical_size.x as f32,
                viewport.physical_size.y as f32,
                viewport.depth.start,
                viewport.depth.end,
            );
        };
        self
    }

    /// Insert a single debug marker.
    ///
    /// This is a GPU debugging feature. This has no effect on the rendering itself.
    pub fn insert_debug_marker(&mut self, label: &str) -> &mut Self {
        detailed_trace!("insert debug marker: {}", label);
        self.pass.insert_debug_marker(label);
        self
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
    pub fn push_debug_group(&mut self, label: &str) -> &mut Self {
        detailed_trace!("push_debug_group marker: {}", label);
        self.pass.push_debug_group(label);
        self
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
    pub fn pop_debug_group(&mut self) -> &mut Self {
        detailed_trace!("pop_debug_group");
        self.pass.pop_debug_group();
        self
    }

    /// Sets the blend color as used by some of the blending modes.
    ///
    /// Subsequent blending tests will test against this value.
    pub fn set_blend_constant(&mut self, color: Color) -> &mut Self {
        detailed_trace!("set blend constant: {:?}", color);
        self.pass.set_blend_constant(wgpu::Color::from(color));
        self
    }

    /// Executes all render commands of all phase items of the given `render_phase`.
    pub fn render_phase<P: PhaseItem>(
        &mut self,
        render_phase: &RenderPhase<P>,
        world: &'a World,
    ) -> &mut Self {
        let draw_functions = world.resource::<DrawFunctions<P>>();
        let mut draw_functions = draw_functions.write();
        draw_functions.prepare(world);

        for item in &render_phase.items {
            let draw_function = draw_functions.get_mut(item.draw_function()).unwrap();
            draw_function.draw(world, self, self.view_entity, item);
        }
        self
    }
}

/// Builder for the [`TrackedRenderPass`].
///
/// This builder is used to configure the color, depth, and stencil attachments of newly created
/// render passes.
///
/// Use the [`begin`](TrackedRenderPassBuilder::begin) method to finalize the builder and
/// start recording render commands.
pub struct TrackedRenderPassBuilder<'v> {
    render_context: &'v mut RenderContext,
    view_entity: Entity,
    label: Option<&'v str>,
    color_attachments: Vec<Option<RenderPassColorAttachment<'v>>>,
    depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'v>>,
}

impl<'v> TrackedRenderPassBuilder<'v> {
    pub(crate) fn new(render_context: &'v mut RenderContext, view_entity: Entity) -> Self {
        Self {
            render_context,
            view_entity,
            label: None,
            color_attachments: vec![],
            depth_stencil_attachment: None,
        }
    }

    /// Sets the optional debug label of the render pass.
    #[inline]
    pub fn set_label(mut self, label: &'v str) -> Self {
        self.label = Some(label);
        self
    }

    /// Adds the view `target` as the next color attachment.
    ///
    /// This uses the default color ops: `store` = true, `load_op` = clear black
    #[inline]
    pub fn add_view_target(mut self, target: &'v ViewTarget) -> Self {
        self.color_attachments
            .push(Some(target.get_color_attachment(Operations::default())));
        self
    }

    /// Adds the view 'target' as the next "unsampled" color attachment.
    ///
    /// This uses the default color ops: `store` = true, `load_op` = clear black
    #[inline]
    pub fn add_view_target_unsampled(mut self, target: &'v ViewTarget) -> Self {
        self.color_attachments.push(Some(
            target.get_unsampled_color_attachment(Operations::default()),
        ));
        self
    }

    /// Adds the `view` as the next color attachment.
    ///
    /// This uses the default color ops: `store` = true, `load_op` = clear black
    #[inline]
    pub fn add_color_attachment(mut self, view: &'v TextureView) -> Self {
        self.color_attachments.push(Some(RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: Operations::default(),
        }));
        self
    }

    /// Adds the 'view' as the depth/stencil attachment.
    ///
    /// The depth and stencil ops are set to `None` by default and should be configured manually.
    #[inline]
    pub fn set_depth_stencil_attachment(mut self, view: &'v TextureView) -> Self {
        self.depth_stencil_attachment = Some(RenderPassDepthStencilAttachment {
            view,
            depth_ops: None,
            stencil_ops: None,
        });
        self
    }

    /// Sets the color operations of the most recently added color attachment.
    #[inline]
    pub fn set_color_ops(mut self, load: LoadOp<wgpu::Color>, store: bool) -> Self {
        let attachment = self.color_attachments.last_mut().unwrap().as_mut().unwrap();
        attachment.ops = Operations { load, store };
        self
    }

    /// Sets the depth operations of the depth/stencil attachment.
    #[inline]
    pub fn set_depth_ops(mut self, load: LoadOp<f32>, store: bool) -> Self {
        let attachment = self.depth_stencil_attachment.as_mut().unwrap();
        attachment.depth_ops = Some(Operations { load, store });
        self
    }

    /// Sets the stencil operations of the depth/stencil attachment.
    #[inline]
    pub fn set_stencil_ops(mut self, load: LoadOp<u32>, store: bool) -> Self {
        let attachment = self.depth_stencil_attachment.as_mut().unwrap();
        attachment.stencil_ops = Some(Operations { load, store });
        self
    }

    /// Finishes the configuration and begins the render pass.
    #[inline]
    pub fn begin(self) -> TrackedRenderPass<'v> {
        let descriptor = RenderPassDescriptor {
            label: self.label,
            color_attachments: &self.color_attachments,
            depth_stencil_attachment: self.depth_stencil_attachment,
        };

        self.render_context
            .begin_tracked_render_pass(descriptor, self.view_entity)
    }
}
