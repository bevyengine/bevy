use bevy_ecs::world::World;

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipelineDescriptor, PipelineCache,
        RenderPipelineDescriptor, SpecializedComputePipeline, SpecializedComputePipelines,
        SpecializedMeshPipeline, SpecializedMeshPipelines, SpecializedRenderPipeline,
        SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
};

use super::resource::IntoRenderResource;

pub struct SpecializeRenderPipeline<'a, P: SpecializedRenderPipeline>(pub &'a P, pub P::Key)
where
    <P as SpecializedRenderPipeline>::Key: Send + Sync;

impl<'a, P: SpecializedRenderPipeline> IntoRenderResource for SpecializeRenderPipeline<'a, P>
where
    <P as SpecializedRenderPipeline>::Key: Send + Sync,
{
    type Resource = CachedRenderPipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        let specializer = world.resource::<SpecializedRenderPipelines<P>>();
        let pipelines = world.resource::<PipelineCache>();
        specializer.specialize(pipelines, self.0, self.1)
    }
}

pub struct SpecializeComputePipeline<'a, P: SpecializedComputePipeline>(pub &'a P, pub P::Key)
where
    <P as SpecializedComputePipeline>::Key: Send + Sync;

impl<'a, P: SpecializedComputePipeline> IntoRenderResource for SpecializeComputePipeline<'a, P>
where
    <P as SpecializedComputePipeline>::Key: Send + Sync,
{
    type Resource = CachedComputePipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        let specializer = world.resource::<SpecializedComputePipelines<P>>();
        let pipelines = world.resource::<PipelineCache>();
        specializer.specialize(pipelines, self.0, self.1)
    }
}

pub struct SpecializeMeshPipeline<'a, P: SpecializedMeshPipeline>(
    pub &'a P,
    pub P::Key,
    pub &'a MeshVertexBufferLayoutRef,
)
where
    <P as SpecializedMeshPipeline>::Key: Send + Sync;

impl<'a, P: SpecializedMeshPipeline> IntoRenderResource for SpecializeMeshPipeline<'a, P>
where
    <P as SpecializedMeshPipeline>::Key: Send + Sync,
{
    type Resource = CachedRenderPipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        let specializer = world.resource::<SpecializedMeshPipelines<P>>();
        let pipelines = world.resource::<PipelineCache>();
        specializer
            .specialize(pipelines, self.0, self.1, self.2)
            .unwrap() //todo: actual error handling!
    }
}

impl IntoRenderResource for RenderPipelineDescriptor {
    type Resource = CachedRenderPipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        let pipelines = world.resource::<PipelineCache>();
        pipelines.queue_render_pipeline(self)
    }
}

impl IntoRenderResource for ComputePipelineDescriptor {
    type Resource = CachedComputePipelineId;

    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource {
        let pipelines = world.resource::<PipelineCache>();
        pipelines.queue_compute_pipeline(self)
    }
}
