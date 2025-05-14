use wgpu::CommandEncoder;

use crate::frame_graph::{
    ComputePassCommand, ComputePassCommandBuilder, ComputePassInfo, FrameGraphError, RenderContext,
};

use super::EncoderExecutor;

#[derive(Default)]
pub struct ComputePass {
    compute_pass: ComputePassInfo,
    commands: Vec<ComputePassCommand>,
}

impl ComputePass {
    pub fn is_vaild(&self) -> bool {
        !self.commands.is_empty()
    }

    pub fn pass_name(&self) -> Option<&str> {
        self.compute_pass.label.as_deref()
    }

    pub fn set_pass_name(&mut self, name: &str) {
        self.compute_pass.label = Some(name.to_string().into());
    }

    pub fn finish(&mut self) {}
}

impl ComputePassCommandBuilder for ComputePass {
    fn add_compute_pass_command(&mut self, value: ComputePassCommand) {
        self.commands.push(value);
    }
}

impl EncoderExecutor for ComputePass {
    fn execute(
        &self,
        command_encoder: &mut CommandEncoder,
        render_context: &mut RenderContext,
    ) -> Result<(), FrameGraphError> {
        let render_pass_context =
            render_context.begin_compute_pass(command_encoder, &self.compute_pass)?;

        render_pass_context.execute(&self.commands)?;

        Ok(())
    }
}
