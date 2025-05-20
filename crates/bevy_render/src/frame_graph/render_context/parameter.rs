use std::ops::Range;

use bevy_color::LinearRgba;
use wgpu::{Extent3d, ImageSubresourceRange, QuerySet, ShaderStages};

use crate::{
    frame_graph::{
        BindGroupBinding, FrameGraphBuffer, FrameGraphTexture, RenderPassContext, ResourceRead,
        ResourceRef, ResourceWrite, TexelCopyBufferInfo, TexelCopyTextureInfo,
    },
    render_resource::{BindGroup, CachedComputePipelineId, CachedRenderPipelineId},
};

use super::{
    encoder_pass_context::{EncoderPassContext, ErasedEncoderPassCommand},
    ComputePassContext, ErasedComputePassCommand, ErasedEncoderCommand, ErasedRenderPassCommand,
};

pub struct DispatchWorkgroupsParameter {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl ErasedComputePassCommand for DispatchWorkgroupsParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.dispatch_workgroups(self.x, self.y, self.z);
    }
}

pub struct ClearBufferParameter {
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceWrite>,
    pub offset: u64,
    pub size: Option<u64>,
}

impl ErasedRenderPassCommand for ClearBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.clear_buffer(&self.buffer_ref, self.offset, self.size);
    }
}

impl ErasedComputePassCommand for ClearBufferParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.clear_buffer(&self.buffer_ref, self.offset, self.size);
    }
}

impl ErasedEncoderPassCommand for ClearBufferParameter {
    fn draw(&self, command_encoder_context: &mut EncoderPassContext) {
        command_encoder_context.clear_buffer(&self.buffer_ref, self.offset, self.size);
    }
}

pub struct ClearTextureParameter {
    pub texture_ref: ResourceRef<FrameGraphTexture, ResourceWrite>,
    pub subresource_range: ImageSubresourceRange,
}

impl ErasedComputePassCommand for ClearTextureParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.clear_texture(&self.texture_ref, &self.subresource_range);
    }
}

impl ErasedRenderPassCommand for ClearTextureParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.clear_texture(&self.texture_ref, &self.subresource_range);
    }
}

impl ErasedEncoderPassCommand for ClearTextureParameter {
    fn draw(&self, command_encoder_context: &mut EncoderPassContext) {
        command_encoder_context.clear_texture(&self.texture_ref, &self.subresource_range);
    }
}

pub struct CopyTextureToBufferParameter {
    pub source: TexelCopyTextureInfo<ResourceRead>,
    pub destination: TexelCopyBufferInfo<ResourceWrite>,
    pub copy_size: Extent3d,
}

impl ErasedComputePassCommand for CopyTextureToBufferParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.copy_texture_to_buffer(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        );
    }
}

impl ErasedRenderPassCommand for CopyTextureToBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.copy_texture_to_buffer(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        );
    }
}

impl ErasedEncoderPassCommand for CopyTextureToBufferParameter {
    fn draw(&self, command_encoder_context: &mut EncoderPassContext) {
        command_encoder_context.copy_texture_to_buffer(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        );
    }
}

pub struct CopyTextureToTextureParameter {
    pub source: TexelCopyTextureInfo<ResourceRead>,
    pub destination: TexelCopyTextureInfo<ResourceWrite>,
    pub copy_size: Extent3d,
}

impl ErasedComputePassCommand for CopyTextureToTextureParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.copy_texture_to_texture(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        );
    }
}

impl ErasedRenderPassCommand for CopyTextureToTextureParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.copy_texture_to_texture(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        );
    }
}

impl ErasedEncoderPassCommand for CopyTextureToTextureParameter {
    fn draw(&self, command_encoder_context: &mut EncoderPassContext) {
        command_encoder_context.copy_texture_to_texture(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        );
    }
}

pub struct DispatchWorkgroupsIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
}

impl ErasedComputePassCommand for DispatchWorkgroupsIndirectParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context
            .dispatch_workgroups_indirect(&self.indirect_buffer_ref, self.indirect_offset);
    }
}

pub struct DrawIndexedIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
}

impl ErasedRenderPassCommand for DrawIndexedIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.draw_indexed_indirect(&self.indirect_buffer_ref, self.indirect_offset);
    }
}

pub struct MultiDrawIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
    pub count: u32,
}

impl ErasedRenderPassCommand for MultiDrawIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.multi_draw_indirect(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            self.count,
        );
    }
}

pub struct MultiDrawIndirectCountParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
    pub count_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub count_offset: u64,
    pub max_count: u32,
}

impl ErasedRenderPassCommand for MultiDrawIndirectCountParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.multi_draw_indexed_indirect_count(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            &self.count_buffer_ref,
            self.count_offset,
            self.max_count,
        );
    }
}

pub struct MultiDrawIndexedIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
    pub count: u32,
}

impl ErasedRenderPassCommand for MultiDrawIndexedIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.multi_draw_indexed_indirect(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            self.count,
        );
    }
}

pub struct MultiDrawIndexedIndirectCountParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
    pub count_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub count_offset: u64,
    pub max_count: u32,
}

impl ErasedRenderPassCommand for MultiDrawIndexedIndirectCountParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.multi_draw_indexed_indirect_count(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            &self.count_buffer_ref,
            self.count_offset,
            self.max_count,
        );
    }
}

pub struct SetStencilReferenceParameter {
    pub reference: u32,
}

impl ErasedRenderPassCommand for SetStencilReferenceParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_stencil_reference(self.reference);
    }
}

pub struct SetPushConstantsParameter {
    pub stages: ShaderStages,
    pub offset: u32,
    pub data: Vec<u8>,
}

pub struct SetPushConstantsComputeParameter {
    pub offset: u32,
    pub data: Vec<u8>,
}

impl ErasedComputePassCommand for SetPushConstantsComputeParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.set_push_constants(self.offset, &self.data);
    }
}

impl ErasedRenderPassCommand for SetPushConstantsParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_push_constants(self.stages.clone(), self.offset, &self.data);
    }
}

pub struct SetViewportParameter {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl ErasedRenderPassCommand for SetViewportParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_viewport(
            self.x,
            self.y,
            self.width,
            self.height,
            self.min_depth,
            self.max_depth,
        );
    }
}

pub struct InsertDebugMarkerParameter {
    pub label: String,
}

impl ErasedComputePassCommand for InsertDebugMarkerParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.insert_debug_marker(&self.label);
    }
}

impl ErasedRenderPassCommand for InsertDebugMarkerParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.insert_debug_marker(&self.label);
    }
}

pub struct PushDebugGroupParameter {
    pub label: String,
}
impl ErasedEncoderCommand for PushDebugGroupParameter {
    fn draw(&self, command_encoder: &mut wgpu::CommandEncoder) {
        command_encoder.push_debug_group(&self.label);
    }
}

impl ErasedComputePassCommand for PushDebugGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.push_debug_group(&self.label);
    }
}

impl ErasedRenderPassCommand for PushDebugGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.push_debug_group(&self.label);
    }
}

pub struct PopDebugGroupParameter;

impl ErasedEncoderCommand for PopDebugGroupParameter {
    fn draw(&self, command_encoder: &mut wgpu::CommandEncoder) {
        command_encoder.pop_debug_group();
    }
}

impl ErasedComputePassCommand for PopDebugGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.pop_debug_group();
    }
}

impl ErasedRenderPassCommand for PopDebugGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.pop_debug_group();
    }
}

pub struct SetBlendConstantParameter {
    pub color: LinearRgba,
}

impl ErasedRenderPassCommand for SetBlendConstantParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_blend_constant(self.color.clone());
    }
}

pub struct WriteTimestampParameter {
    pub query_set: QuerySet,
    pub index: u32,
}

impl ErasedComputePassCommand for WriteTimestampParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.write_timestamp(&self.query_set, self.index);
    }
}

impl ErasedRenderPassCommand for WriteTimestampParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.write_timestamp(&self.query_set, self.index);
    }
}

pub struct BeginPipelineStatisticsQueryParameter {
    pub query_set: QuerySet,
    pub index: u32,
}

impl ErasedComputePassCommand for BeginPipelineStatisticsQueryParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.write_timestamp(&self.query_set, self.index);
    }
}

impl ErasedRenderPassCommand for BeginPipelineStatisticsQueryParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.begin_pipeline_statistics_query(&self.query_set, self.index);
    }
}

pub struct EndPipelineStatisticsQueryParameter;

impl ErasedComputePassCommand for EndPipelineStatisticsQueryParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.end_pipeline_statistics_query();
    }
}

impl ErasedRenderPassCommand for EndPipelineStatisticsQueryParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.end_pipeline_statistics_query();
    }
}

pub struct DrawIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
}

impl ErasedRenderPassCommand for DrawIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.draw_indirect(&self.indirect_buffer_ref, self.indirect_offset);
    }
}

pub struct DrawIndexedParameter {
    pub indices: Range<u32>,
    pub base_vertex: i32,
    pub instances: Range<u32>,
}

impl ErasedRenderPassCommand for DrawIndexedParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.draw_indexed(
            self.indices.clone(),
            self.base_vertex,
            self.instances.clone(),
        );
    }
}

pub struct SetRawBindGroupParameter {
    pub index: u32,
    pub bind_group: Option<BindGroup>,
    pub offsets: Vec<u32>,
}

impl ErasedComputePassCommand for SetRawBindGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.set_raw_bind_group(
            self.index,
            self.bind_group.as_ref(),
            &self.offsets,
        );
    }
}

impl ErasedRenderPassCommand for SetRawBindGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_raw_bind_group(self.index, self.bind_group.as_ref(), &self.offsets);
    }
}

pub struct SetScissorRectParameter {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ErasedRenderPassCommand for SetScissorRectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_scissor_rect(self.x, self.y, self.width, self.height);
    }
}

pub struct DrawParameter {
    pub vertices: Range<u32>,
    pub instances: Range<u32>,
}

impl ErasedRenderPassCommand for DrawParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.draw(self.vertices.clone(), self.instances.clone());
    }
}

pub struct SetIndexBufferParameter {
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub index_format: wgpu::IndexFormat,
    pub offset: u64,
    pub size: u64,
}

impl ErasedRenderPassCommand for SetIndexBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_index_buffer(
            &self.buffer_ref,
            self.index_format,
            self.offset,
            self.size,
        );
    }
}

pub struct SetVertexBufferParameter {
    pub slot: u32,
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub offset: u64,
    pub size: u64,
}

impl ErasedRenderPassCommand for SetVertexBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_vertex_buffer(self.slot, &self.buffer_ref, self.offset, self.size);
    }
}

pub struct SetComputePipelineParameter {
    pub id: CachedComputePipelineId,
}

impl ErasedComputePassCommand for SetComputePipelineParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.set_compute_pipeline(self.id);
    }
}

pub struct SetRenderPipelineParameter {
    pub id: CachedRenderPipelineId,
}

impl ErasedRenderPassCommand for SetRenderPipelineParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_render_pipeline(self.id);
    }
}

pub struct SetBindGroupParameter {
    pub index: u32,
    pub bind_group: BindGroupBinding,
    pub offsets: Vec<u32>,
}

impl ErasedComputePassCommand for SetBindGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) {
        compute_pass_context.set_bind_group(self.index, &self.bind_group, &self.offsets);
    }
}

impl ErasedRenderPassCommand for SetBindGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) {
        render_pass_context.set_bind_group(self.index, &self.bind_group, &self.offsets);
    }
}
