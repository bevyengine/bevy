use crate::{
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, RenderPipeline, RenderPipelineDescriptor,
    },
    PipelineCache as PipelineCompiler,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct PipelineCache {
    compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
    render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
}

impl PipelineCache {
    pub fn get_or_compile_compute_pipeline(
        &mut self,
        descriptor: ComputePipelineDescriptor,
        pipeline_compiler: &PipelineCompiler,
    ) -> Option<ComputePipeline> {
        let pipeline_id = *self
            .compute_pipelines
            .entry(descriptor.clone())
            .or_insert_with(|| pipeline_compiler.queue_compute_pipeline(descriptor));

        pipeline_compiler.get_compute_pipeline(pipeline_id).cloned()
    }

    pub fn get_or_compile_render_pipeline(
        &mut self,
        descriptor: RenderPipelineDescriptor,
        pipeline_compiler: &PipelineCompiler,
    ) -> Option<RenderPipeline> {
        let pipeline_id = *self
            .render_pipelines
            .entry(descriptor.clone())
            .or_insert_with(|| pipeline_compiler.queue_render_pipeline(descriptor));

        pipeline_compiler.get_render_pipeline(pipeline_id).cloned()
    }
}
