use wgpu::{Extent3d, ImageSubresourceRange};

use crate::frame_graph::{
    TransientBuffer, TransientTexture, ResourceRead, Ref, ResourceWrite,
    TexelCopyBufferInfo, TexelCopyTextureInfo,
};

use super::{
    ClearBufferParameter, ClearTextureParameter, CopyTextureToBufferParameter, CopyTextureToTextureParameter, RenderContext
};

pub trait EncoderPassCommandBuilder {
    fn add_encoder_pass_command(&mut self, value: EncoderPassCommand);

     fn copy_texture_to_buffer(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyBufferInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) {
        self.add_encoder_pass_command(EncoderPassCommand::new(CopyTextureToBufferParameter {
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
        self.add_encoder_pass_command(EncoderPassCommand::new(ClearBufferParameter {
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
        self.add_encoder_pass_command(EncoderPassCommand::new(ClearTextureParameter {
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
        self.add_encoder_pass_command(EncoderPassCommand::new(CopyTextureToTextureParameter {
            source,
            destination,
            copy_size,
        }));
    }
}

pub struct EncoderPassCommand(Box<dyn ErasedEncoderPassCommand>);

impl EncoderPassCommand {
    pub fn new<T: ErasedEncoderPassCommand>(value: T) -> Self {
        Self(Box::new(value))
    }

    pub fn draw(&self, command_encoder_context: &mut EncoderPassContext) {
        self.0.draw(command_encoder_context)
    }
}

pub trait ErasedEncoderPassCommand: Sync + Send + 'static {
    fn draw(&self, command_encoder_context: &mut EncoderPassContext);
}

pub struct EncoderPassContext<'a, 'b> {
    command_encoder: &'b mut wgpu::CommandEncoder,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> EncoderPassContext<'a, 'b> {
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

    pub fn execute(mut self, commands: &Vec<EncoderPassCommand>) {
        for command in commands {
            command.draw(&mut self);
        }
    }

    pub fn new(
        command_encoder: &'b mut wgpu::CommandEncoder,
        render_context: &'b mut RenderContext<'a>,
    ) -> Self {
        EncoderPassContext {
            command_encoder,
            render_context,
        }
    }
}
