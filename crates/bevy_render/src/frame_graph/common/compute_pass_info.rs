use std::borrow::Cow;

use crate::frame_graph::FrameGraphError;

#[derive(Default)]
pub struct ComputePassInfo {
    pub label: Option<Cow<'static, str>>,
}

impl ComputePassInfo {
    pub fn create_render_pass(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::ComputePass<'static>, FrameGraphError> {
        let compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: self.label.as_deref(),
            timestamp_writes: None,
        });

        let compute_pass = compute_pass.forget_lifetime();

        Ok(compute_pass)
    }
}
