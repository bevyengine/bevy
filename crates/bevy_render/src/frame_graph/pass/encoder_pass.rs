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
        let encoder_context = render_context.begin_encoder();

        encoder_context.execute(&self.commands)?;

        Ok(())
    }
}
