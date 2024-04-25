use bevy_ecs::{
    system::{Res, ResMut, Resource, SystemState},
    world::World,
};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_graph_v2::{seal, RenderGraph},
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, PipelineCache, RenderPipeline, RenderPipelineDescriptor,
        SpecializedComputePipeline, SpecializedComputePipelines, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, SpecializedMeshPipelines, SpecializedRenderPipeline,
        SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
};

use super::{
    CachedRenderStore, IntoRenderResource, RenderResource, RenderResourceInit, RenderResourceMeta,
};

impl seal::Super for RenderPipeline {}

impl RenderResource for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type Data = CachedRenderPipelineId;
    type Store = CachedRenderStore<Self>;

    fn get_store(graph: &RenderGraph, _: seal::Token) -> &Self::Store {
        &graph.render_pipelines
    }

    fn get_store_mut(graph: &mut RenderGraph, _: seal::Token) -> &mut Self::Store {
        &mut graph.render_pipelines
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        world.resource::<PipelineCache>().get_render_pipeline(*data)
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        _render_device: &RenderDevice,
    ) -> Self::Data {
        world
            .resource::<PipelineCache>()
            .queue_render_pipeline(descriptor.clone())
    }
}

impl IntoRenderResource for RenderPipelineDescriptor {
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}

impl seal::Super for ComputePipeline {}

impl RenderResource for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;
    type Data = CachedComputePipelineId;
    type Store = CachedRenderStore<Self>;

    fn get_store(graph: &RenderGraph, _: seal::Token) -> &Self::Store {
        &graph.compute_pipelines
    }

    fn get_store_mut(graph: &mut RenderGraph, _: seal::Token) -> &mut Self::Store {
        &mut graph.compute_pipelines
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        world
            .resource::<PipelineCache>()
            .get_compute_pipeline(*data)
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        _render_device: &RenderDevice,
    ) -> Self::Data {
        world
            .resource::<PipelineCache>()
            .queue_compute_pipeline(descriptor.clone())
    }
}

impl IntoRenderResource for ComputePipelineDescriptor {
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}

pub struct SpecializeRenderPipeline<P: SpecializedRenderPipeline + Resource + 'static>(pub P::Key)
where
    P::Key: Send + Sync + 'static;

impl<P: SpecializedRenderPipeline + Resource + Send + Sync + 'static> IntoRenderResource
    for SpecializeRenderPipeline<P>
where
    P::Key: Send + Sync + 'static,
{
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::FromDescriptor(world.resource::<P>().specialize(self.0))
    }
}

pub struct SpecializeComputePipeline<P: SpecializedComputePipeline + Resource + 'static>(
    pub P::Key,
)
where
    P::Key: Send + Sync + 'static;

impl<P: SpecializedComputePipeline + Resource + 'static> IntoRenderResource
    for SpecializeComputePipeline<P>
where
    P::Key: Send + Sync + 'static,
{
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::FromDescriptor(world.resource::<P>().specialize(self.0))
    }
}

pub struct SpecializeMeshPipeline<P: SpecializedMeshPipeline + Resource + 'static>(
    pub MeshVertexBufferLayoutRef,
    pub P::Key,
)
where
    P::Key: Send + Sync + 'static;

impl<P: SpecializedMeshPipeline + Resource + Send + Sync + 'static> IntoRenderResource
    for SpecializeMeshPipeline<P>
where
    P::Key: Send + Sync + 'static,
{
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::FromDescriptor(
            world
                .resource::<P>()
                .specialize(self.1, &self.0)
                .unwrap_or_else(|err| panic!("{}", err)),
        )
    }
}
