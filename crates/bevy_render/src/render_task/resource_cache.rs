use crate::{
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, IntoBindingArray, RenderPipeline, RenderPipelineDescriptor,
    },
    PipelineCache as PipelineCompiler,
};
use bevy_ecs::entity::Entity;
use std::collections::HashMap;
use wgpu::{BindGroup, Buffer, BufferDescriptor, TextureDescriptor, TextureView};

#[derive(Default)]
pub struct ResourceCache {
    compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
    render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
    textures: HashMap<(Entity, TextureDescriptor<'static>), TextureView>,
    buffers: HashMap<(Entity, BufferDescriptor<'static>), Buffer>,
}

impl ResourceCache {
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

    pub fn get_or_create_texture(&mut self) -> TextureView {
        todo!()
    }

    pub fn get_or_create_buffer(&mut self) -> Buffer {
        todo!()
    }

    pub fn get_or_create_bind_group<'b, const N: usize>(
        &mut self,
        resources: impl IntoBindingArray<'b, N>,
    ) -> BindGroup {
        todo!()
    }
}
