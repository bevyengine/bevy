use crate::{WgpuRenderContext, bind_group::{self, Pass}, resources::WgpuResourceRefs};
use bevy_render2::{
    pass::ComputePass,
    pipeline::{BindGroupDescriptorId, ComputePipelineDescriptor, PipelineId},
    render_resource::BindGroupId,
    renderer::RenderContext,
};

#[derive(Debug)]
pub struct WgpuComputePass<'a> {
    pub compute_pass: wgpu::ComputePass<'a>,
    pub render_context: &'a WgpuRenderContext,
    pub wgpu_resources: WgpuResourceRefs<'a>,
    pub pipeline_descriptor: Option<&'a ComputePipelineDescriptor>,
}

impl<'a> ComputePass for WgpuComputePass<'a> {
    fn get_render_context(&self) -> &dyn RenderContext {
        self.render_context
    }

    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<&[u32]>,
    ) {
        bind_group::set_bind_group(
            Pass::Compute(&mut self.compute_pass),
            &self.wgpu_resources,
            index,
            bind_group_descriptor_id,
            bind_group,
            dynamic_uniform_indices,
        )
    }

    fn set_pipeline(&mut self, pipeline: PipelineId) {
        let pipeline = self
            .wgpu_resources
            .compute_pipelines
            .get(&pipeline)
            .expect(
            "Attempted to use a pipeline that does not exist in this `RenderPass`'s `RenderContext`.",
        );
        self.compute_pass.set_pipeline(pipeline);
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.compute_pass.dispatch(x, y, z);
    }
}
