use wgpu::Extent3d;

use crate::frame_graph::{FrameGraphError, TexelCopyTextureInfo};

use super::{CopyTextureToTextureParameter, RenderContext};

pub trait CommandEncoderCommandBuilder {
    fn add_render_pass_command(&mut self, value: CommandEncoderCommand);

    fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo,
        destination: TexelCopyTextureInfo,
        copy_size: Extent3d,
    ) {
        self.add_render_pass_command(CommandEncoderCommand::new(CopyTextureToTextureParameter {
            source,
            destination,
            copy_size,
        }));
    }
}

pub struct CommandEncoderCommand(Box<dyn ErasedCommandEncoderCommand>);

impl CommandEncoderCommand {
    pub fn new<T: ErasedCommandEncoderCommand>(value: T) -> Self {
        Self(Box::new(value))
    }

    pub fn draw(
        &self,
        command_encoder_context: &mut CommandEncoderContext,
    ) -> Result<(), FrameGraphError> {
        self.0.draw(command_encoder_context)
    }
}

pub trait ErasedCommandEncoderCommand: Sync + Send + 'static {
    fn draw(
        &self,
        command_encoder_context: &mut CommandEncoderContext,
    ) -> Result<(), FrameGraphError>;
}

pub struct CommandEncoderContext<'a, 'b> {
    command_encoder: wgpu::CommandEncoder,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> CommandEncoderContext<'a, 'b> {
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

    pub fn execute(mut self, commands: &Vec<CommandEncoderCommand>) -> Result<(), FrameGraphError> {
        for command in commands {
            command.draw(&mut self)?;
        }

        Ok(())
    }

    pub fn new(
        command_encoder: wgpu::CommandEncoder,
        render_context: &'b mut RenderContext<'a>,
    ) -> Self {
        CommandEncoderContext {
            command_encoder,
            render_context,
        }
    }
}
