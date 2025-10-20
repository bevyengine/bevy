use crate::{
    render_resource::{
        BindGroup, BindGroupEntries, CachedComputePipelineId, CachedRenderPipelineId,
        ComputePipeline, ComputePipelineDescriptor, IntoBindingArray, RenderPipeline,
        RenderPipelineDescriptor,
    },
    renderer::RenderDevice,
    PipelineCache as PipelineCompiler,
};
use bevy_ecs::entity::Entity;
use std::collections::HashMap;
use wgpu::{Buffer, BufferDescriptor, TextureDescriptor, TextureView};

#[derive(Default)]
pub struct ResourceCache {
    textures: HashMap<(Entity, TextureDescriptor<'static>), TextureView>,
    buffers: HashMap<(Entity, BufferDescriptor<'static>), Buffer>,
    compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
    render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
}

impl ResourceCache {
    pub fn get_or_create_texture(&mut self, render_device: &RenderDevice) -> TextureView {
        todo!()
    }

    pub fn get_or_create_buffer(&mut self, render_device: &RenderDevice) -> Buffer {
        todo!()
    }

    pub fn get_or_create_bind_group<'b, const N: usize>(
        &mut self,
        resources: impl IntoBindingArray<'b, N>,
        render_device: &RenderDevice,
    ) -> BindGroup {
        render_device.create_bind_group("TODO", todo!(), &BindGroupEntries::sequential(resources))
    }

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
