use std::ops::Range;

use bevy_color::LinearRgba;
use wgpu::{Extent3d, ImageSubresourceRange, QuerySet, ShaderStages};

use crate::{
    frame_graph::{
        BindGroupDrawing, FrameGraphBuffer, FrameGraphError, FrameGraphTexture, RenderPassContext,
        ResourceRead, ResourceRef, ResourceWrite, TexelCopyTextureInfo,
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
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.dispatch_workgroups(self.x, self.y, self.z);
        Ok(())
    }
}

pub struct ClearTextureParameter {
    pub texture_ref: ResourceRef<FrameGraphTexture, ResourceWrite>,
    pub subresource_range: ImageSubresourceRange,
}

impl ErasedComputePassCommand for ClearTextureParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.clear_texture(&self.texture_ref, &self.subresource_range)?;
        Ok(())
    }
}

impl ErasedRenderPassCommand for ClearTextureParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.clear_texture(&self.texture_ref, &self.subresource_range)?;
        Ok(())
    }
}

impl ErasedEncoderPassCommand for ClearTextureParameter {
    fn draw(
        &self,
        command_encoder_context: &mut EncoderPassContext,
    ) -> Result<(), FrameGraphError> {
        command_encoder_context.clear_texture(&self.texture_ref, &self.subresource_range)?;
        Ok(())
    }
}

pub struct CopyTextureToTextureParameter {
    pub source: TexelCopyTextureInfo<ResourceRead>,
    pub destination: TexelCopyTextureInfo<ResourceWrite>,
    pub copy_size: Extent3d,
}

impl ErasedComputePassCommand for CopyTextureToTextureParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.copy_texture_to_texture(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        )?;
        Ok(())
    }
}

impl ErasedRenderPassCommand for CopyTextureToTextureParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.copy_texture_to_texture(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        )?;
        Ok(())
    }
}

impl ErasedEncoderPassCommand for CopyTextureToTextureParameter {
    fn draw(&self, command_encoder_context: &mut EncoderPassContext) -> Result<(), FrameGraphError> {
        command_encoder_context.copy_texture_to_texture(
            self.source.clone(),
            self.destination.clone(),
            self.copy_size.clone(),
        )?;
        Ok(())
    }
}

pub struct DrawIndexedIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
}

impl ErasedRenderPassCommand for DrawIndexedIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context
            .draw_indexed_indirect(&self.indirect_buffer_ref, self.indirect_offset)?;
        Ok(())
    }
}

pub struct MultiDrawIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
    pub count: u32,
}

impl ErasedRenderPassCommand for MultiDrawIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.multi_draw_indirect(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            self.count,
        )?;
        Ok(())
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
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.multi_draw_indexed_indirect_count(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            &self.count_buffer_ref,
            self.count_offset,
            self.max_count,
        )?;
        Ok(())
    }
}

pub struct MultiDrawIndexedIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
    pub count: u32,
}

impl ErasedRenderPassCommand for MultiDrawIndexedIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.multi_draw_indexed_indirect(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            self.count,
        )?;
        Ok(())
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
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.multi_draw_indexed_indirect_count(
            &self.indirect_buffer_ref,
            self.indirect_offset,
            &self.count_buffer_ref,
            self.count_offset,
            self.max_count,
        )?;
        Ok(())
    }
}

pub struct SetStencilReferenceParameter {
    pub reference: u32,
}

impl ErasedRenderPassCommand for SetStencilReferenceParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_stencil_reference(self.reference);
        Ok(())
    }
}

pub struct SetPushConstantsParameter {
    pub stages: ShaderStages,
    pub offset: u32,
    pub data: Vec<u8>,
}

impl ErasedRenderPassCommand for SetPushConstantsParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_push_constants(self.stages.clone(), self.offset, &self.data);
        Ok(())
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
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_viewport(
            self.x,
            self.y,
            self.width,
            self.height,
            self.min_depth,
            self.max_depth,
        );
        Ok(())
    }
}

pub struct InsertDebugMarkerParameter {
    pub label: String,
}

impl ErasedComputePassCommand for InsertDebugMarkerParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.insert_debug_marker(&self.label);
        Ok(())
    }
}

impl ErasedRenderPassCommand for InsertDebugMarkerParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.insert_debug_marker(&self.label);
        Ok(())
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
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.push_debug_group(&self.label);
        Ok(())
    }
}

impl ErasedRenderPassCommand for PushDebugGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.push_debug_group(&self.label);
        Ok(())
    }
}

pub struct PopDebugGroupParameter;

impl ErasedEncoderCommand for PopDebugGroupParameter {
    fn draw(&self, command_encoder: &mut wgpu::CommandEncoder) {
        command_encoder.pop_debug_group();
    }
}

impl ErasedComputePassCommand for PopDebugGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.pop_debug_group();
        Ok(())
    }
}

impl ErasedRenderPassCommand for PopDebugGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.pop_debug_group();
        Ok(())
    }
}

pub struct SetBlendConstantParameter {
    pub color: LinearRgba,
}

impl ErasedRenderPassCommand for SetBlendConstantParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_blend_constant(self.color.clone());
        Ok(())
    }
}

pub struct WriteTimestampParameter {
    pub query_set: QuerySet,
    pub index: u32,
}

impl ErasedComputePassCommand for WriteTimestampParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.write_timestamp(&self.query_set, self.index);
        Ok(())
    }
}

impl ErasedRenderPassCommand for WriteTimestampParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.write_timestamp(&self.query_set, self.index);
        Ok(())
    }
}

pub struct BeginPipelineStatisticsQueryParameter {
    pub query_set: QuerySet,
    pub index: u32,
}

impl ErasedComputePassCommand for BeginPipelineStatisticsQueryParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.write_timestamp(&self.query_set, self.index);
        Ok(())
    }
}

impl ErasedRenderPassCommand for BeginPipelineStatisticsQueryParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.begin_pipeline_statistics_query(&self.query_set, self.index);
        Ok(())
    }
}

pub struct EndPipelineStatisticsQueryParameter;

impl ErasedComputePassCommand for EndPipelineStatisticsQueryParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.end_pipeline_statistics_query();
        Ok(())
    }
}

impl ErasedRenderPassCommand for EndPipelineStatisticsQueryParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.end_pipeline_statistics_query();
        Ok(())
    }
}

pub struct DrawIndirectParameter {
    pub indirect_buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub indirect_offset: u64,
}

impl ErasedRenderPassCommand for DrawIndirectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.draw_indirect(&self.indirect_buffer_ref, self.indirect_offset)?;
        Ok(())
    }
}

pub struct DrawIndexedParameter {
    pub indices: Range<u32>,
    pub base_vertex: i32,
    pub instances: Range<u32>,
}

impl ErasedRenderPassCommand for DrawIndexedParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.draw_indexed(
            self.indices.clone(),
            self.base_vertex,
            self.instances.clone(),
        );
        Ok(())
    }
}

pub struct SetRawBindGroupParameter {
    pub index: u32,
    pub bind_group: Option<BindGroup>,
    pub offsets: Vec<u32>,
}

impl ErasedComputePassCommand for SetRawBindGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.set_raw_bind_group(
            self.index,
            self.bind_group.as_ref(),
            &self.offsets,
        )?;
        Ok(())
    }
}

impl ErasedRenderPassCommand for SetRawBindGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_raw_bind_group(
            self.index,
            self.bind_group.as_ref(),
            &self.offsets,
        )?;
        Ok(())
    }
}

pub struct SetScissorRectParameter {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ErasedRenderPassCommand for SetScissorRectParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_scissor_rect(self.x, self.y, self.width, self.height);
        Ok(())
    }
}

pub struct DrawParameter {
    pub vertices: Range<u32>,
    pub instances: Range<u32>,
}

impl ErasedRenderPassCommand for DrawParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.draw(self.vertices.clone(), self.instances.clone());
        Ok(())
    }
}

pub struct SetIndexBufferParameter {
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub index_format: wgpu::IndexFormat,
    pub offset: u64,
    pub size: u64,
}

impl ErasedRenderPassCommand for SetIndexBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_index_buffer(
            &self.buffer_ref,
            self.index_format,
            self.offset,
            self.size,
        )?;
        Ok(())
    }
}

pub struct SetVertexBufferParameter {
    pub slot: u32,
    pub buffer_ref: ResourceRef<FrameGraphBuffer, ResourceRead>,
    pub offset: u64,
    pub size: u64,
}

impl ErasedRenderPassCommand for SetVertexBufferParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_vertex_buffer(
            self.slot,
            &self.buffer_ref,
            self.offset,
            self.size,
        )?;
        Ok(())
    }
}

pub struct SetComputePipelineParameter {
    pub id: CachedComputePipelineId,
}

impl ErasedComputePassCommand for SetComputePipelineParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.set_compute_pipeline(self.id)?;

        Ok(())
    }
}

pub struct SetRenderPipelineParameter {
    pub id: CachedRenderPipelineId,
}

impl ErasedRenderPassCommand for SetRenderPipelineParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_render_pipeline(self.id)?;
        Ok(())
    }
}

pub struct SetBindGroupParameter {
    pub index: u32,
    pub bind_group: BindGroupDrawing,
    pub offsets: Vec<u32>,
}

impl ErasedComputePassCommand for SetBindGroupParameter {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError> {
        compute_pass_context.set_bind_group(self.index, &self.bind_group, &self.offsets)?;

        Ok(())
    }
}

impl ErasedRenderPassCommand for SetBindGroupParameter {
    fn draw(&self, render_pass_context: &mut RenderPassContext) -> Result<(), FrameGraphError> {
        render_pass_context.set_bind_group(self.index, &self.bind_group, &self.offsets)?;
        Ok(())
    }
}
