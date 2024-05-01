use bevy_ecs::{system::Resource, world::World};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{
        ComputePipeline, ComputePipelineDescriptor, RenderPipeline, RenderPipelineDescriptor,
        SpecializedComputePipeline, SpecializedMeshPipeline, SpecializedRenderPipeline,
    },
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, IntoRenderResource, NewRenderResource, RenderHandle,
    RenderResource,
};

impl RenderResource for RenderPipeline {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl DescribedRenderResource for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Option<Self::Descriptor>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_descriptor<'g>(
        graph: &RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'g Self::Descriptor> {
        todo!()
    }
}

impl<'g> IntoRenderResource<'g> for RenderPipelineDescriptor {
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(self))
    }
}

impl RenderResource for ComputePipeline {
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl DescribedRenderResource for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;

    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Option<Self::Descriptor>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_descriptor<'g>(
        graph: &RenderGraph<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'g Self::Descriptor> {
        todo!()
    }
}

impl<'g> IntoRenderResource<'g> for ComputePipelineDescriptor {
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}

pub struct SpecializeRenderPipeline<P: SpecializedRenderPipeline + Resource>(pub P::Key);

impl<'g, P: SpecializedRenderPipeline + Resource> IntoRenderResource<'g>
    for SpecializeRenderPipeline<P>
{
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}

pub struct SpecializeComputePipeline<P: SpecializedComputePipeline + Resource>(pub P::Key);

impl<'g, P: SpecializedComputePipeline + Resource> IntoRenderResource<'g>
    for SpecializeComputePipeline<P>
{
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
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
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        todo!()
    }
}
