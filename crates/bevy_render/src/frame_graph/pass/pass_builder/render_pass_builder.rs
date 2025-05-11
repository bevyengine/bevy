use std::{mem::take, ops::Range};

use bevy_color::LinearRgba;
use tracing::warn;
use wgpu::{QuerySet, ShaderStages};

use crate::{
    camera::Viewport,
    frame_graph::{
        BindGroupDrawing, ColorAttachment, ColorAttachmentDrawing, DepthStencilAttachmentDrawing,
        FrameGraphBuffer, FrameGraphTexture, RenderPass, RenderPassCommandBuilder, ResourceHandle,
        ResourceRead, ResourceRef,
    },
    render_resource::{BindGroup, Buffer, CachedRenderPipelineId, Texture},
};

use super::PassBuilder;

pub struct RenderPassBuilder<'a, 'b> {
    render_pass: RenderPass,
    pass_builder: &'b mut PassBuilder<'a>,
}

impl<'a, 'b> Drop for RenderPassBuilder<'a, 'b> {
    fn drop(&mut self) {
        self.finish();
    }
}

impl<'a, 'b> RenderPassBuilder<'a, 'b> {
    pub fn new(pass_builder: &'b mut PassBuilder<'a>) -> Self {
        let render_pass = RenderPass::default();

        Self {
            render_pass,
            pass_builder,
        }
    }

    pub fn import_and_read_buffer(
        &mut self,
        buffer: &Buffer,
    ) -> ResourceRef<FrameGraphBuffer, ResourceRead> {
        self.pass_builder
            .pass_node_builder()
            .import_and_read_buffer(buffer)
    }

    pub fn import_and_read_texture(
        &mut self,
        texture: &Texture,
    ) -> ResourceRef<FrameGraphTexture, ResourceRead> {
        self.pass_builder
            .pass_node_builder()
            .import_and_read_texture(texture)
    }

    pub fn set_bind_group<T>(&mut self, index: u32, bind_group: T, offsets: &[u32]) -> &mut Self
    where
        T: ResourceHandle<Drawing = BindGroupDrawing>,
    {
        let bind_group =
            bind_group.make_resource_drawing(&mut self.pass_builder.pass_node_builder());

        self.render_pass.set_bind_group(index, &bind_group, offsets);
        self
    }

    pub fn create_render_pass_builder(&mut self) -> &mut Self {
        self.finish();

        self
    }

    fn finish(&mut self) {
        self.render_pass.finish();

        let render_pass = take(&mut self.render_pass);

        if render_pass.is_vaild() {
            self.pass_builder.add_executor(render_pass);
        } else {
            warn!("render pass must is vaild");
        }
    }

    pub fn set_pass_name(&mut self, name: &str) -> &mut Self {
        self.render_pass.set_pass_name(name);

        self
    }

    pub fn end_pipeline_statistics_query(&mut self) -> &mut Self {
        self.render_pass.end_pipeline_statistics_query();

        self
    }

    pub fn begin_pipeline_statistics_query(
        &mut self,
        query_set: &QuerySet,
        index: u32,
    ) -> &mut Self {
        self.render_pass
            .begin_pipeline_statistics_query(query_set, index);

        self
    }

    pub fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) -> &mut Self {
        self.render_pass.write_timestamp(query_set, index);

        self
    }

    pub fn set_blend_constant(&mut self, color: LinearRgba) -> &mut Self {
        self.render_pass.set_blend_constant(color);

        self
    }

    pub fn pop_debug_group(&mut self) -> &mut Self {
        self.render_pass.pop_debug_group();

        self
    }

    pub fn push_debug_group(&mut self, label: &str) -> &mut Self {
        self.render_pass.push_debug_group(label);

        self
    }

    pub fn insert_debug_marker(&mut self, label: &str) -> &mut Self {
        self.render_pass.insert_debug_marker(label);

        self
    }

    pub fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) -> &mut Self {
        self.render_pass
            .set_viewport(x, y, width, height, min_depth, max_depth);

        self
    }

    pub fn set_push_constants(
        &mut self,
        stages: ShaderStages,
        offset: u32,
        data: &[u8],
    ) -> &mut Self {
        self.render_pass.set_push_constants(stages, offset, data);

        self
    }

    pub fn set_stencil_reference(&mut self, reference: u32) -> &mut Self {
        self.render_pass.set_stencil_reference(reference);

        self
    }

    pub fn multi_draw_indexed_indirect_count(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
        count_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        count_offset: u64,
        max_count: u32,
    ) -> &mut Self {
        self.render_pass.multi_draw_indexed_indirect_count(
            indirect_buffer_ref,
            indirect_offset,
            count_buffer_ref,
            count_offset,
            max_count,
        );

        self
    }

    pub fn multi_draw_indexed_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
        count: u32,
    ) -> &mut Self {
        self.render_pass
            .multi_draw_indexed_indirect(indirect_buffer_ref, indirect_offset, count);

        self
    }

    pub fn multi_draw_indirect_count(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
        count_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        count_offset: u64,
        max_count: u32,
    ) -> &mut Self {
        self.render_pass.multi_draw_indexed_indirect_count(
            indirect_buffer_ref,
            indirect_offset,
            count_buffer_ref,
            count_offset,
            max_count,
        );

        self
    }

    pub fn multi_draw_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
        count: u32,
    ) -> &mut Self {
        self.render_pass
            .multi_draw_indirect(indirect_buffer_ref, indirect_offset, count);

        self
    }

    pub fn draw_indexed_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
    ) -> &mut Self {
        self.render_pass
            .draw_indexed_indirect(indirect_buffer_ref, indirect_offset);
        self
    }

    pub fn draw_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
    ) -> &mut Self {
        self.render_pass
            .draw_indirect(indirect_buffer_ref, indirect_offset);
        self
    }

    pub fn draw_indexed(
        &mut self,
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    ) -> &mut Self {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);

        self
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) -> &mut Self {
        self.render_pass.draw(vertices, instances);
        self
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
        offset: u64,
        size: u64,
    ) -> &mut Self {
        self.render_pass
            .set_index_buffer(buffer_ref, index_format, offset, size);

        self
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        offset: u64,
        szie: u64,
    ) -> &mut Self {
        self.render_pass
            .set_vertex_buffer(slot, buffer_ref, offset, szie);
        self
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) -> &mut Self {
        self.render_pass
            .set_raw_bind_group(index, bind_group, offsets);

        self
    }

    pub fn set_camera_viewport(&mut self, viewport: Option<Viewport>) -> &mut Self {
        self.render_pass.set_camera_viewport(viewport);

        self
    }

    pub fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) -> &mut Self {
        self.render_pass.set_render_pipeline(id);
        self
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
        self.render_pass.set_scissor_rect(x, y, width, height);
        self
    }

    pub fn add_raw_color_attachment(&mut self, color_attachment: ColorAttachment) -> &mut Self {
        self.render_pass.add_raw_color_attachment(color_attachment);
        self
    }

    pub fn add_color_attachment(&mut self, color_attachment: ColorAttachmentDrawing) -> &mut Self {
        self.render_pass.add_color_attachment(color_attachment);

        self
    }

    pub fn set_depth_stencil_attachment(
        &mut self,
        depth_stencil_attachment: DepthStencilAttachmentDrawing,
    ) -> &mut Self {
        self.render_pass
            .set_depth_stencil_attachment(depth_stencil_attachment);

        self
    }
}
