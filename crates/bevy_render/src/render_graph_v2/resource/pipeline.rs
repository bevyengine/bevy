use bevy_ecs::{system::Resource, world::World};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_graph_v2::{seal, RenderGraph},
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, RenderPipeline, RenderPipelineDescriptor,
        SpecializedComputePipeline, SpecializedMeshPipeline, SpecializedRenderPipeline,
    },
    renderer::RenderDevice,
};

use super::{CachedRenderStore, IntoRenderResource, RenderResource, RenderResourceInit};

impl seal::Super for RenderPipeline {}

impl RenderResource for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type Data = CachedRenderPipelineId;
    type Store<'g> = CachedRenderStore<'g, Self>;

    fn get_store<'a, 'g: 'a>(graph: &'a RenderGraph<'g>, _: seal::Token) -> &'a Self::Store<'g> {
        todo!()
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        todo!()
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        todo!()
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
        todo!()
    }

    fn get_store_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraph<'g>,
        _: seal::Token,
    ) -> &'a mut Self::Store<'g> {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        todo!()
    }

    fn from_descriptor(
        descriptor: &Self::Descriptor,
        world: &World,
        render_device: &RenderDevice,
    ) -> Self::Data {
        todo!()
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
