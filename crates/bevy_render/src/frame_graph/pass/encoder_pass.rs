use wgpu::CommandEncoder;

use crate::frame_graph::{
    encoder_context::{EncoderPassCommand, EncoderPassCommandBuilder},
    FrameGraphError, RenderContext,
};

use super::EncoderExecutor;

#[derive(Default)]
pub struct EncoderPass {
    commands: Vec<EncoderPassCommand>,
}

impl EncoderPassCommandBuilder for EncoderPass {
    fn add_encoder_pass_command(&mut self, value: EncoderPassCommand) {
        self.commands.push(value);
    }
}

impl EncoderExecutor for EncoderPass {
    fn execute(
        &self,
        command_encoder: &mut CommandEncoder,
        render_context: &mut RenderContext,
    ) -> Result<(), FrameGraphError> {
        let encoder_context = render_context.begin_encoder(command_encoder);

        encoder_context.execute(&self.commands)?;

        Ok(())
    }
}
