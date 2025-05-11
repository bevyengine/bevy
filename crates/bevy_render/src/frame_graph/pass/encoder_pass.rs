use crate::frame_graph::{
    encoder_context::{EncoderPassCommand, EncoderPassCommandBuilder},
    FrameGraphError, RenderContext,
};

use super::PassTrait;

#[derive(Default)]
pub struct EncoderPass {
    commands: Vec<EncoderPassCommand>,
}

impl EncoderPassCommandBuilder for EncoderPass {
    fn add_encoder_pass_command(&mut self, value: EncoderPassCommand) {
        self.commands.push(value);
    }
}

impl PassTrait for EncoderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        render_context.flush_encoder();

        let mut command_encoder = render_context.create_command_encoder();

        let encoder_context = render_context.begin_encoder(&mut command_encoder);

        encoder_context.execute(&self.commands)?;

        Ok(())
    }
}
