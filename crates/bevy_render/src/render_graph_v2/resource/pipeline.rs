use bevy_ecs::{system::Resource, world::World};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_graph_v2::{seal, RenderGraph, RenderGraphPersistentResources},
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, PipelineCache, RenderPipeline, RenderPipelineDescriptor,
        SpecializedComputePipeline, SpecializedMeshPipeline, SpecializedRenderPipeline,
    },
    renderer::RenderDevice,
};

use super::{
    CachedRenderStore, IntoRenderResource, RenderResource, RenderResourceInit, RenderStore,
};

impl seal::Super for RenderPipeline {}

impl RenderResource for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type Data = CachedRenderPipelineId;
    type Store<'g> = CachedRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        &graph.render_pipelines
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        &mut graph.render_pipelines
    }

    fn get_persistent_store(
        persistent_resources: &RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &<Self::Store<'static> as RenderStore<'static, Self>>::PersistentStore {
        &persistent_resources.render_pipelines
    }

    fn get_persistent_store_mut<'g>(
        persistent_resources: &mut RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &mut <Self::Store<'static> as RenderStore<'static, Self>>::PersistentStore {
        &mut persistent_resources.render_pipelines
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

impl<'g> IntoRenderResource<'g> for RenderPipelineDescriptor {
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}

impl seal::Super for ComputePipeline {}

impl RenderResource for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;
    type Data = CachedComputePipelineId;
    type Store<'g> = CachedRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        &graph.compute_pipelines
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        &mut graph.compute_pipelines
    }

    fn get_persistent_store(
        persistent_resources: &RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &<Self::Store<'static> as RenderStore<'static, Self>>::PersistentStore {
        &persistent_resources.compute_pipelines
    }

    fn get_persistent_store_mut<'g>(
        persistent_resources: &mut RenderGraphPersistentResources,
        _: seal::Token,
    ) -> &mut <Self::Store<'static> as RenderStore<'static, Self>>::PersistentStore {
        &mut persistent_resources.compute_pipelines
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

impl<'g> IntoRenderResource<'g> for ComputePipelineDescriptor {
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(self)
    }
}

pub struct SpecializeRenderPipeline<P: SpecializedRenderPipeline + Resource>(pub P::Key);

impl<'g, P: SpecializedRenderPipeline + Resource> IntoRenderResource<'g>
    for SpecializeRenderPipeline<P>
{
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(world.resource::<P>().specialize(self.0))
    }
}

pub struct SpecializeComputePipeline<P: SpecializedComputePipeline + Resource>(pub P::Key);

impl<'g, P: SpecializedComputePipeline + Resource> IntoRenderResource<'g>
    for SpecializeComputePipeline<P>
{
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(world.resource::<P>().specialize(self.0))
    }
}

pub struct SpecializeMeshPipeline<P: SpecializedMeshPipeline + Resource + 'static>(
    pub MeshVertexBufferLayoutRef,
    pub P::Key,
);

impl<'g, P: SpecializedMeshPipeline + Resource> IntoRenderResource<'g>
    for SpecializeMeshPipeline<P>
{
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<'g, Self::Resource> {
        RenderResourceInit::FromDescriptor(
            world
                .resource::<P>()
                .specialize(self.1, &self.0)
                .unwrap_or_else(|err| panic!("{}", err)),
        )
    }
}
