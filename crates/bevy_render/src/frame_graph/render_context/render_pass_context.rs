use bevy_color::LinearRgba;
use wgpu::{Extent3d, ImageSubresourceRange, QuerySet, ShaderStages};

use super::{
    BeginPipelineStatisticsQueryParameter, ClearBufferParameter, ClearTextureParameter,
    CopyTextureToBufferParameter, CopyTextureToTextureParameter, DrawIndexedIndirectParameter,
    DrawIndexedParameter, DrawIndirectParameter, DrawParameter,
    EndPipelineStatisticsQueryParameter, TransientBuffer, InsertDebugMarkerParameter,
    MultiDrawIndexedIndirectCountParameter, MultiDrawIndexedIndirectParameter,
    MultiDrawIndirectParameter, PopDebugGroupParameter, PushDebugGroupParameter, RenderContext,
    ResourceRead, Ref, SetBindGroupParameter, SetBlendConstantParameter,
    SetIndexBufferParameter, SetPushConstantsParameter, SetRawBindGroupParameter,
    SetRenderPipelineParameter, SetScissorRectParameter, SetStencilReferenceParameter,
    SetVertexBufferParameter, SetViewportParameter, WriteTimestampParameter,
};
use crate::{
    frame_graph::{
        BindGroupBinding, TransientTexture, ResourceBinding, ResourceWrite, TexelCopyBufferInfo,
        TexelCopyTextureInfo,
    },
    render_resource::{BindGroup, CachedRenderPipelineId},
};
use core::ops::Range;
use std::ops::Deref;

pub trait RenderPassCommandBuilder {
    fn add_render_pass_command(&mut self, value: RenderPassCommand);

    fn copy_texture_to_buffer(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyBufferInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(CopyTextureToBufferParameter {
            source,
            destination,
            copy_size,
        }));
    }

    fn clear_buffer(
        &mut self,
        buffer_ref: &Ref<TransientBuffer, ResourceWrite>,
        offset: u64,
        size: Option<u64>,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(ClearBufferParameter {
            buffer_ref: buffer_ref.clone(),
            offset,
            size,
        }));
    }

    fn clear_texture(
        &mut self,
        texture_ref: &Ref<TransientTexture, ResourceWrite>,
        subresource_range: ImageSubresourceRange,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(ClearTextureParameter {
            texture_ref: texture_ref.clone(),
            subresource_range,
        }));
    }

    fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyTextureInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(CopyTextureToTextureParameter {
            source,
            destination,
            copy_size,
        }));
    }

    fn draw_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(DrawIndirectParameter {
            indirect_buffer_ref: indirect_buffer_ref.clone(),
            indirect_offset,
        }));
    }

    fn draw_indexed_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(DrawIndexedIndirectParameter {
            indirect_buffer_ref: indirect_buffer_ref.clone(),
            indirect_offset,
        }));
    }

    fn multi_draw_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count: u32,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(MultiDrawIndirectParameter {
            indirect_buffer_ref: indirect_buffer_ref.clone(),
            indirect_offset,
            count,
        }));
    }

    fn multi_draw_indirect_count(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        count_offset: u64,
        max_count: u32,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(
            MultiDrawIndexedIndirectCountParameter {
                indirect_buffer_ref: indirect_buffer_ref.clone(),
                indirect_offset,
                count_buffer_ref: count_buffer_ref.clone(),
                count_offset,
                max_count,
            },
        ));
    }

    fn multi_draw_indexed_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count: u32,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(MultiDrawIndexedIndirectParameter {
            indirect_buffer_ref: indirect_buffer_ref.clone(),
            indirect_offset,
            count,
        }));
    }

    fn multi_draw_indexed_indirect_count(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        count_offset: u64,
        max_count: u32,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(
            MultiDrawIndexedIndirectCountParameter {
                indirect_buffer_ref: indirect_buffer_ref.clone(),
                indirect_offset,
                count_buffer_ref: count_buffer_ref.clone(),
                count_offset,
                max_count,
            },
        ));
    }

    fn set_stencil_reference(&mut self, reference: u32) {
        self.add_render_pass_command(RenderPassCommand::new(SetStencilReferenceParameter {
            reference,
        }));
    }

    fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        self.add_render_pass_command(RenderPassCommand::new(SetPushConstantsParameter {
            stages,
            offset,
            data: data.to_vec(),
        }));
    }

    fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(SetViewportParameter {
            x,
            y,
            width,
            height,
            min_depth,
            max_depth,
        }));
    }

    fn insert_debug_marker(&mut self, label: &str) {
        self.add_render_pass_command(RenderPassCommand::new(InsertDebugMarkerParameter {
            label: label.to_string(),
        }));
    }

    fn push_debug_group(&mut self, label: &str) {
        self.add_render_pass_command(RenderPassCommand::new(PushDebugGroupParameter {
            label: label.to_string(),
        }));
    }

    fn pop_debug_group(&mut self) {
        self.add_render_pass_command(RenderPassCommand::new(PopDebugGroupParameter));
    }

    fn set_blend_constant(&mut self, color: LinearRgba) {
        self.add_render_pass_command(RenderPassCommand::new(SetBlendConstantParameter { color }));
    }

    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        self.add_render_pass_command(RenderPassCommand::new(WriteTimestampParameter {
            query_set: query_set.clone(),
            index,
        }));
    }

    fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        self.add_render_pass_command(RenderPassCommand::new(
            BeginPipelineStatisticsQueryParameter {
                query_set: query_set.clone(),
                index,
            },
        ));
    }

    fn end_pipeline_statistics_query(&mut self) {
        self.add_render_pass_command(RenderPassCommand::new(EndPipelineStatisticsQueryParameter));
    }

    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.add_render_pass_command(RenderPassCommand::new(DrawIndexedParameter {
            indices,
            base_vertex,
            instances,
        }));
    }

    fn set_raw_bind_group(&mut self, index: u32, bind_group: Option<&BindGroup>, offsets: &[u32]) {
        self.add_render_pass_command(RenderPassCommand::new(SetRawBindGroupParameter {
            index,
            bind_group: bind_group.map(|bind_group| bind_group.clone()),
            offsets: offsets.to_vec(),
        }));
    }

    fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.add_render_pass_command(RenderPassCommand::new(SetScissorRectParameter {
            x,
            y,
            width,
            height,
        }));
    }

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.add_render_pass_command(RenderPassCommand::new(DrawParameter {
            vertices,
            instances,
        }));
    }

    fn set_index_buffer(
        &mut self,
        buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
        offset: u64,
        size: u64,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(SetIndexBufferParameter {
            buffer_ref: buffer_ref.clone(),
            index_format,
            offset,
            size,
        }));
    }

    fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        self.add_render_pass_command(RenderPassCommand::new(SetRenderPipelineParameter { id }));
    }

    fn set_bind_group(&mut self, index: u32, bind_group: &BindGroupBinding, offsets: &[u32]) {
        self.add_render_pass_command(RenderPassCommand::new(SetBindGroupParameter {
            index,
            bind_group: bind_group.clone(),
            offsets: offsets.to_vec(),
        }));
    }

    fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        offset: u64,
        size: u64,
    ) {
        self.add_render_pass_command(RenderPassCommand::new(SetVertexBufferParameter {
            slot,
            buffer_ref: buffer_ref.clone(),
            offset,
            size,
        }));
    }
}

pub struct RenderPassCommand(Box<dyn ErasedRenderPassCommand>);

impl RenderPassCommand {
    pub fn new<T: ErasedRenderPassCommand>(value: T) -> Self {
        Self(Box::new(value))
    }

    pub fn draw(&self, render_pass_context: &mut RenderPassContext) {
        self.0.draw(render_pass_context)
    }
}

pub trait ErasedRenderPassCommand: Sync + Send + 'static {
    fn draw(&self, render_pass_context: &mut RenderPassContext);
}

pub struct RenderPassContext<'a, 'b> {
    command_encoder: &'b mut wgpu::CommandEncoder,
    render_pass: wgpu::RenderPass<'b>,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> RenderPassContext<'a, 'b> {
    pub fn new(
        command_encoder: &'b mut wgpu::CommandEncoder,
        render_pass: wgpu::RenderPass<'b>,
        render_context: &'b mut RenderContext<'a>,
    ) -> Self {
        RenderPassContext {
            command_encoder,
            render_pass,
            render_context,
        }
    }

    pub fn copy_texture_to_buffer(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyBufferInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) {
        let source_texture = self.render_context.get_resource(&source.texture);
        let destination_buffer = self.render_context.get_resource(&destination.buffer);

        self.command_encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfoBase {
                texture: &source_texture.resource,
                mip_level: source.mip_level,
                origin: source.origin,
                aspect: source.aspect,
            },
            wgpu::TexelCopyBufferInfoBase {
                buffer: &destination_buffer.resource,
                layout: destination.layout,
            },
            copy_size,
        );
    }

    pub fn clear_buffer(
        &mut self,
        buffer_ref: &Ref<TransientBuffer, ResourceWrite>,
        offset: u64,
        size: Option<u64>,
    ) {
        let buffer = self.render_context.get_resource(&buffer_ref);

        self.command_encoder
            .clear_buffer(&buffer.resource, offset, size);
    }

    pub fn clear_texture(
        &mut self,
        texture_ref: &Ref<TransientTexture, ResourceWrite>,
        subresource_range: &ImageSubresourceRange,
    ) {
        let texture = self.render_context.get_resource(&texture_ref);

        self.command_encoder
            .clear_texture(&texture.resource, subresource_range);
    }

    pub fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyTextureInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) {
        let source_texture = self.render_context.get_resource(&source.texture);
        let destination_texture = self.render_context.get_resource(&destination.texture);

        self.command_encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &source_texture.resource,
                mip_level: source.mip_level,
                origin: source.origin,
                aspect: source.aspect,
            },
            wgpu::TexelCopyTextureInfoBase {
                texture: &destination_texture.resource,
                mip_level: destination.mip_level,
                origin: destination.origin,
                aspect: destination.aspect,
            },
            copy_size,
        );
    }

    pub fn end_pipeline_statistics_query(&mut self) {
        self.render_pass.end_pipeline_statistics_query();
    }

    pub fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        self.render_pass
            .begin_pipeline_statistics_query(query_set, index);
    }

    pub fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        self.render_pass.write_timestamp(query_set, index);
    }

    pub fn set_blend_constant(&mut self, color: LinearRgba) {
        self.render_pass
            .set_blend_constant(wgpu::Color::from(color));
    }

    pub fn pop_debug_group(&mut self) {
        self.render_pass.pop_debug_group();
    }

    pub fn push_debug_group(&mut self, label: &str) {
        self.render_pass.push_debug_group(label);
    }

    pub fn insert_debug_marker(&mut self, label: &str) {
        self.render_pass.insert_debug_marker(label);
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
        self.render_pass
            .set_viewport(x, y, width, height, min_depth, max_depth);
    }

    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        self.render_pass.set_push_constants(stages, offset, data);
    }

    pub fn set_stencil_reference(&mut self, reference: u32) {
        self.render_pass.set_stencil_reference(reference);
    }

    pub fn multi_draw_indexed_indirect_count(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        count_offset: u64,
        max_count: u32,
    ) {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref);
        let count_buffer = self.render_context.get_resource(count_buffer_ref);

        self.render_pass.multi_draw_indexed_indirect_count(
            &indirect_buffer.resource,
            indirect_offset,
            &count_buffer.resource,
            count_offset,
            max_count,
        );
    }

    pub fn multi_draw_indexed_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count: u32,
    ) {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref);

        self.render_pass.multi_draw_indexed_indirect(
            &indirect_buffer.resource,
            indirect_offset,
            count,
        );
    }

    pub fn multi_draw_indirect_count(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        count_offset: u64,
        max_count: u32,
    ) {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref);
        let count_buffer = self.render_context.get_resource(count_buffer_ref);

        self.render_pass.multi_draw_indirect_count(
            &indirect_buffer.resource,
            indirect_offset,
            &count_buffer.resource,
            count_offset,
            max_count,
        );
    }

    pub fn multi_draw_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
        count: u32,
    ) {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref);

        self.render_pass
            .multi_draw_indirect(&indirect_buffer.resource, indirect_offset, count);
    }

    pub fn draw_indexed_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
    ) {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref);

        self.render_pass
            .draw_indexed_indirect(&indirect_buffer.resource, indirect_offset);
    }

    pub fn draw_indirect(
        &mut self,
        indirect_buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        indirect_offset: u64,
    ) {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref);

        self.render_pass
            .draw_indirect(&indirect_buffer.resource, indirect_offset);
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.render_pass.set_scissor_rect(x, y, width, height);
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) {
        self.render_pass.set_bind_group(
            index,
            bind_group.map(|bind_group| bind_group.deref()),
            offsets,
        );
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group: &BindGroupBinding, offsets: &[u32]) {
        let bind_group = bind_group.make_resource(&self.render_context);
        self.render_pass.set_bind_group(index, &bind_group, offsets);
    }

    pub fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        let pipeline = self.render_context.get_render_pipeline(id);
        self.render_pass.set_pipeline(pipeline);
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances);
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        offset: u64,
        size: u64,
    ) {
        let buffer = self.render_context.get_resource(buffer_ref);
        self.render_pass
            .set_vertex_buffer(slot, buffer.resource.slice(offset..(offset + size)));
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &Ref<TransientBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
        offset: u64,
        size: u64,
    ) {
        let buffer = self.render_context.get_resource(buffer_ref);

        self.render_pass
            .set_index_buffer(buffer.resource.slice(offset..(offset + size)), index_format);
    }

    pub fn execute(mut self, commands: &Vec<RenderPassCommand>) {
        for command in commands {
            command.draw(&mut self);
        }
    }
}
