use crate::frame_graph::{
    command_encoder_context::{CommandEncoderCommand, CommandEncoderCommandBuilder},
    FrameGraphError, RenderContext,
};

use super::PassTrait;

#[derive(Default)]
pub struct CommandEncoderPass {
    commands: Vec<CommandEncoderCommand>,
}

impl CommandEncoderCommandBuilder for CommandEncoderPass {
    fn add_render_pass_command(&mut self, value: CommandEncoderCommand) {
        self.commands.push(value);
    }
}

impl PassTrait for CommandEncoderPass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        let command_encoder_context = render_context.begin_command_encoder();

        command_encoder_context.execute(&self.commands)?;

        Ok(())
    }
}
