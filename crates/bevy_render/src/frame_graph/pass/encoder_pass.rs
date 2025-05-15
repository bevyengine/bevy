use wgpu::CommandEncoder;

use crate::frame_graph::{
    encoder_pass_context::{EncoderPassCommand, EncoderPassCommandBuilder},
    RenderContext,
};

use super::EncoderExecutor;

#[derive(Default)]
pub struct EncoderPass {
    commands: Vec<EncoderPassCommand>,
}

impl EncoderPass {
    pub fn is_vaild(&self) -> bool {
        !self.commands.is_empty()
    }

    pub fn finish(&mut self) {}
}

impl EncoderPassCommandBuilder for EncoderPass {
    fn add_encoder_pass_command(&mut self, value: EncoderPassCommand) {
        self.commands.push(value);
    }
}

impl EncoderExecutor for EncoderPass {
    fn execute(&self, command_encoder: &mut CommandEncoder, render_context: &mut RenderContext) {
        let encoder_context = render_context.begin_encoder_pass(command_encoder);

        encoder_context.execute(&self.commands);
    }
}
