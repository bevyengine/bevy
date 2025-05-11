use crate::frame_graph::{
    ComputePassCommand, ComputePassCommandBuilder, ComputePassInfo, FrameGraphError, RenderContext,
};

use super::PassTrait;

#[derive(Default)]
pub struct ComputePass {
    compute_pass: ComputePassInfo,
    commands: Vec<ComputePassCommand>,
}

impl ComputePass {
    pub fn is_vaild(&self) -> bool {
        !self.commands.is_empty()
    }
}

impl ComputePassCommandBuilder for ComputePass {
    fn add_compute_pass_command(&mut self, value: ComputePassCommand) {
        self.commands.push(value);
    }
}

impl PassTrait for ComputePass {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        render_context.flush_encoder();
        let mut command_encoder = render_context.create_command_encoder();
       
        let render_pass_context = render_context.begin_compute_pass(&mut command_encoder,&self.compute_pass)?;

        render_pass_context.execute(&self.commands)?;

        Ok(())
    }
}
