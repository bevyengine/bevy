use wgpu::{Extent3d, ImageSubresourceRange, QuerySet, ShaderStages};

use super::{
    BeginPipelineStatisticsQueryParameter, ClearBufferParameter, ClearTextureParameter,
    CopyTextureToTextureParameter, DispatchWorkgroupsIndirectParameter,
    DispatchWorkgroupsParameter, EndPipelineStatisticsQueryParameter, FrameGraphError,
    InsertDebugMarkerParameter, PopDebugGroupParameter, PushDebugGroupParameter, RenderContext,
    SetBindGroupParameter, SetComputePipelineParameter, SetPushConstantsComputeParameter,
    SetRawBindGroupParameter, WriteTimestampParameter,
};
use crate::{
    frame_graph::{
        BindGroupDrawing, FrameGraphBuffer, FrameGraphTexture, ResourceDrawing, ResourceRead,
        ResourceRef, ResourceWrite, TexelCopyTextureInfo,
    },
    render_resource::{BindGroup, CachedComputePipelineId},
};
use std::ops::Deref;

pub trait ComputePassCommandBuilder {
    fn add_compute_pass_command(&mut self, value: ComputePassCommand);

    fn clear_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceWrite>,
        offset: u64,
        size: Option<u64>,
    ) {
        self.add_compute_pass_command(ComputePassCommand::new(ClearBufferParameter {
            buffer_ref: buffer_ref.clone(),
            offset,
            size,
        }));
    }

    fn dispatch_workgroups_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
    ) {
        self.add_compute_pass_command(ComputePassCommand::new(
            DispatchWorkgroupsIndirectParameter {
                indirect_buffer_ref: indirect_buffer_ref.clone(),
                indirect_offset,
            },
        ));
    }

    fn set_push_constants(&mut self, offset: u32, data: &[u8]) {
        self.add_compute_pass_command(ComputePassCommand::new(SetPushConstantsComputeParameter {
            offset,
            data: data.to_vec(),
        }));
    }

    fn clear_texture(
        &mut self,
        texture_ref: &ResourceRef<FrameGraphTexture, ResourceWrite>,
        subresource_range: ImageSubresourceRange,
    ) {
        self.add_compute_pass_command(ComputePassCommand::new(ClearTextureParameter {
            texture_ref: texture_ref.clone(),
            subresource_range,
        }));
    }

    fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
        self.add_compute_pass_command(ComputePassCommand::new(DispatchWorkgroupsParameter {
            x,
            y,
            z,
        }));
    }

    fn set_compute_pipeline(&mut self, id: CachedComputePipelineId) {
        self.add_compute_pass_command(ComputePassCommand::new(SetComputePipelineParameter { id }));
    }

    fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyTextureInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) {
        self.add_compute_pass_command(ComputePassCommand::new(CopyTextureToTextureParameter {
            source,
            destination,
            copy_size,
        }));
    }

    fn insert_debug_marker(&mut self, label: &str) {
        self.add_compute_pass_command(ComputePassCommand::new(InsertDebugMarkerParameter {
            label: label.to_string(),
        }));
    }

    fn push_debug_group(&mut self, label: &str) {
        self.add_compute_pass_command(ComputePassCommand::new(PushDebugGroupParameter {
            label: label.to_string(),
        }));
    }

    fn pop_debug_group(&mut self) {
        self.add_compute_pass_command(ComputePassCommand::new(PopDebugGroupParameter));
    }

    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        self.add_compute_pass_command(ComputePassCommand::new(WriteTimestampParameter {
            query_set: query_set.clone(),
            index,
        }));
    }

    fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        self.add_compute_pass_command(ComputePassCommand::new(
            BeginPipelineStatisticsQueryParameter {
                query_set: query_set.clone(),
                index,
            },
        ));
    }

    fn end_pipeline_statistics_query(&mut self) {
        self.add_compute_pass_command(ComputePassCommand::new(EndPipelineStatisticsQueryParameter));
    }

    fn set_raw_bind_group(&mut self, index: u32, bind_group: Option<&BindGroup>, offsets: &[u32]) {
        self.add_compute_pass_command(ComputePassCommand::new(SetRawBindGroupParameter {
            index,
            bind_group: bind_group.map(|bind_group| bind_group.clone()),
            offsets: offsets.to_vec(),
        }));
    }

    fn set_bind_group(&mut self, index: u32, bind_group: &BindGroupDrawing, offsets: &[u32]) {
        self.add_compute_pass_command(ComputePassCommand::new(SetBindGroupParameter {
            index,
            bind_group: bind_group.clone(),
            offsets: offsets.to_vec(),
        }));
    }
}

pub struct ComputePassCommand(Box<dyn ErasedComputePassCommand>);

impl ComputePassCommand {
    pub fn new<T: ErasedComputePassCommand>(value: T) -> Self {
        Self(Box::new(value))
    }

    pub fn draw(
        &self,
        compute_pass_context: &mut ComputePassContext,
    ) -> Result<(), FrameGraphError> {
        self.0.draw(compute_pass_context)
    }
}

pub trait ErasedComputePassCommand: Sync + Send + 'static {
    fn draw(&self, compute_pass_context: &mut ComputePassContext) -> Result<(), FrameGraphError>;
}

pub struct ComputePassContext<'a, 'b> {
    command_encoder: &'b mut wgpu::CommandEncoder,
    compute_pass: wgpu::ComputePass<'b>,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> ComputePassContext<'a, 'b> {
    pub fn new(
        command_encoder: &'b mut wgpu::CommandEncoder,
        compute_pass: wgpu::ComputePass<'b>,
        render_context: &'b mut RenderContext<'a>,
    ) -> Self {
        ComputePassContext {
            command_encoder,
            compute_pass,
            render_context,
        }
    }

    pub fn clear_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceWrite>,
        offset: u64,
        size: Option<u64>,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(&buffer_ref)?;

        self.command_encoder
            .clear_buffer(&buffer.resource, offset, size);

        Ok(())
    }

    pub fn dispatch_workgroups_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
    ) -> Result<(), FrameGraphError> {
        let indirect_buffer = self.render_context.get_resource(indirect_buffer_ref)?;

        self.compute_pass
            .dispatch_workgroups_indirect(&indirect_buffer.resource, indirect_offset);

        Ok(())
    }

    pub fn set_push_constants(&mut self, offset: u32, data: &[u8]) {
        self.compute_pass.set_push_constants(offset, data);
    }

    pub fn clear_texture(
        &mut self,
        texture_ref: &ResourceRef<FrameGraphTexture, ResourceWrite>,
        subresource_range: &ImageSubresourceRange,
    ) -> Result<(), FrameGraphError> {
        let texture = self.render_context.get_resource(&texture_ref)?;

        self.command_encoder
            .clear_texture(&texture.resource, subresource_range);

        Ok(())
    }

    pub fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
        self.compute_pass.dispatch_workgroups(x, y, z);
    }

    pub fn set_compute_pipeline(
        &mut self,
        id: CachedComputePipelineId,
    ) -> Result<(), FrameGraphError> {
        let pipeline = self.render_context.get_compute_pipeline(id)?;
        self.compute_pass.set_pipeline(pipeline);

        Ok(())
    }

    pub fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyTextureInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) -> Result<(), FrameGraphError> {
        let source_texture = self.render_context.get_resource(&source.texture)?;
        let destination_texture = self.render_context.get_resource(&destination.texture)?;

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

        Ok(())
    }

    pub fn end_pipeline_statistics_query(&mut self) {
        self.compute_pass.end_pipeline_statistics_query();
    }

    pub fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        self.compute_pass
            .begin_pipeline_statistics_query(query_set, index);
    }

    pub fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        self.compute_pass.write_timestamp(query_set, index);
    }

    pub fn pop_debug_group(&mut self) {
        self.compute_pass.pop_debug_group();
    }

    pub fn push_debug_group(&mut self, label: &str) {
        self.compute_pass.push_debug_group(label);
    }

    pub fn insert_debug_marker(&mut self, label: &str) {
        self.compute_pass.insert_debug_marker(label);
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) -> Result<(), FrameGraphError> {
        self.compute_pass.set_bind_group(
            index,
            bind_group.map(|bind_group| bind_group.deref()),
            offsets,
        );

        Ok(())
    }

    pub fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &BindGroupDrawing,
        offsets: &[u32],
    ) -> Result<(), FrameGraphError> {
        let bind_group = bind_group.make_resource(&self.render_context)?;
        self.compute_pass
            .set_bind_group(index, &bind_group, offsets);

        Ok(())
    }

    pub fn execute(mut self, commands: &Vec<ComputePassCommand>) -> Result<(), FrameGraphError> {
        for command in commands {
            command.draw(&mut self)?;
        }

        Ok(())
    }
}
