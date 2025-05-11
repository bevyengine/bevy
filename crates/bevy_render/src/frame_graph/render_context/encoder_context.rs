use wgpu::Extent3d;

use crate::frame_graph::{FrameGraphError, TexelCopyTextureInfo};

use super::{CopyTextureToTextureParameter, RenderContext};

pub trait EncoderPassCommandBuilder {
    fn add_encoder_pass_command(&mut self, value:EncoderPassCommand);

    fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo,
        destination: TexelCopyTextureInfo,
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

    pub fn draw(
        &self,
        command_encoder_context: &mut EncoderContext,
    ) -> Result<(), FrameGraphError> {
        self.0.draw(command_encoder_context)
    }
}

pub trait ErasedEncoderPassCommand: Sync + Send + 'static {
    fn draw(
        &self,
        command_encoder_context: &mut EncoderContext,
    ) -> Result<(), FrameGraphError>;
}

pub struct EncoderContext<'a, 'b> {
    command_encoder: &'b mut wgpu::CommandEncoder,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> EncoderContext<'a, 'b> {
    pub fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo,
        destination: TexelCopyTextureInfo,
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

    pub fn execute(mut self, commands: &Vec<EncoderPassCommand>) -> Result<(), FrameGraphError> {
        for command in commands {
            command.draw(&mut self)?;
        }

        Ok(())
    }

    pub fn new(
        command_encoder:&'b mut wgpu::CommandEncoder,
        render_context: &'b mut RenderContext<'a>,
    ) -> Self {
       EncoderContext {
            command_encoder,
            render_context,
        }
    }
}
