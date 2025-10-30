use crate::{
    render_resource::{
        BindGroup, BindGroupEntries, Buffer, CachedComputePipelineId, CachedRenderPipelineId,
        ComputePipeline, ComputePipelineDescriptor, IntoBindingArray, RenderPipeline,
        RenderPipelineDescriptor, TextureView,
    },
    renderer::RenderDevice,
    PipelineCache as PipelineCompiler,
};
use bevy_ecs::entity::Entity;
use std::collections::HashMap;
use wgpu::{BufferDescriptor, TextureDescriptor, TextureViewDescriptor};

#[derive(Default)]
pub struct ResourceCache {
    textures: HashMap<(Entity, TextureDescriptor<'static>), TextureView>,
    buffers: HashMap<(Entity, BufferDescriptor<'static>), Buffer>,
    compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
    render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
}

impl ResourceCache {
    pub fn get_or_create_texture(
        &mut self,
        descriptor: TextureDescriptor<'static>,
        entity: Entity,
        render_device: &RenderDevice,
    ) -> TextureView {
        self.textures
            .entry((entity, descriptor.clone()))
            .or_insert_with(|| {
                render_device
                    .create_texture(&descriptor)
                    .create_view(&TextureViewDescriptor::default())
            })
            .clone()
    }

    pub fn get_or_create_buffer(
        &mut self,
        descriptor: BufferDescriptor<'static>,
        entity: Entity,
        render_device: &RenderDevice,
    ) -> Buffer {
        self.buffers
            .entry((entity, descriptor.clone()))
            .or_insert_with(|| render_device.create_buffer(&descriptor))
            .clone()
    }

    pub fn get_or_create_bind_group<'b, const N: usize>(
        &mut self,
        resources: impl IntoBindingArray<'b, N>,
        render_device: &RenderDevice,
    ) -> BindGroup {
        // TODO: Cache
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
