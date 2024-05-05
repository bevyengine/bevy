use std::borrow::Borrow;

use bevy_ecs::system::Resource;
use bevy_utils::HashMap;

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, PipelineCache, RenderPipeline, RenderPipelineDescriptor,
        SpecializedComputePipeline, SpecializedMeshPipeline, SpecializedRenderPipeline,
    },
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, IntoRenderResource, NewRenderResource, RenderHandle,
    RenderResource, RenderResourceId, ResourceTracker,
};

#[derive(Default)]
pub struct CachedRenderGraphPipelines {
    cached_render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
    cached_compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
}

#[derive(Default)]
pub struct RenderGraphPipelines<'g> {
    render_pipelines: HashMap<RenderResourceId, RenderPipelineMeta<'g>>,
    compute_pipelines: HashMap<RenderResourceId, ComputePipelineMeta<'g>>,
}

enum RenderPipelineMeta<'g> {
    Direct(Option<RenderPipelineDescriptor>, RefEq<'g, RenderPipeline>),
    Cached(CachedRenderPipelineId),
}

enum ComputePipelineMeta<'g> {
    Direct(
        Option<ComputePipelineDescriptor>,
        RefEq<'g, ComputePipeline>,
    ),
    Cached(CachedComputePipelineId),
}

//TODO: more complexity for creation, since pipelines need to reference bind group layouts
impl<'g> RenderGraphPipelines<'g> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_direct_render_pipeline(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: Option<RenderPipelineDescriptor>,
        pipeline: RefEq<'g, RenderPipeline>,
    ) -> RenderResourceId {
        let id = tracker.new_resource(None);
        self.render_pipelines
            .insert(id, RenderPipelineMeta::Direct(descriptor, pipeline));
        id
    }

    pub fn new_direct_compute_pipeline(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: Option<ComputePipelineDescriptor>,
        pipeline: RefEq<'g, ComputePipeline>,
    ) -> RenderResourceId {
        let id = tracker.new_resource(None);
        self.compute_pipelines
            .insert(id, ComputePipelineMeta::Direct(descriptor, pipeline));
        id
    }

    pub fn new_cached_render_pipeline(
        &mut self,
        tracker: &mut ResourceTracker,
        cache: &mut CachedRenderGraphPipelines,
        pipeline_cache: &PipelineCache,
        descriptor: RenderPipelineDescriptor,
    ) -> RenderResourceId {
        let id = tracker.new_resource(None);
        let render_pipeline_id = cache
            .cached_render_pipelines
            .entry(descriptor.clone())
            .or_insert_with(|| pipeline_cache.queue_render_pipeline(descriptor));
        self.render_pipelines
            .insert(id, RenderPipelineMeta::Cached(*render_pipeline_id));
        id
    }

    pub fn new_cached_compute_pipeline(
        &mut self,
        tracker: &mut ResourceTracker,
        cache: &mut CachedRenderGraphPipelines,
        pipeline_cache: &PipelineCache,
        descriptor: ComputePipelineDescriptor,
    ) -> RenderResourceId {
        let id = tracker.new_resource(None);
        let compute_pipeline_id = cache
            .cached_compute_pipelines
            .entry(descriptor.clone())
            .or_insert_with(|| pipeline_cache.queue_compute_pipeline(descriptor));
        self.compute_pipelines
            .insert(id, ComputePipelineMeta::Cached(*compute_pipeline_id));
        id
    }

    pub fn get_render_pipeline_descriptor<'a>(
        &'a self,
        cache: &'a PipelineCache,
        id: RenderResourceId,
    ) -> Option<&'a RenderPipelineDescriptor> {
        let meta = self.render_pipelines.get(&id)?;
        match meta {
            RenderPipelineMeta::Direct(descriptor, _) => descriptor.as_ref(),
            RenderPipelineMeta::Cached(pipeline_id) => {
                Some(cache.get_render_pipeline_descriptor(*pipeline_id))
            }
        }
    }

    pub fn get_compute_pipeline_descriptor<'a>(
        &'a self,
        cache: &'a PipelineCache,
        id: RenderResourceId,
    ) -> Option<&'a ComputePipelineDescriptor> {
        let meta = self.compute_pipelines.get(&id)?;
        match meta {
            ComputePipelineMeta::Direct(descriptor, _) => descriptor.as_ref(),
            ComputePipelineMeta::Cached(pipeline_id) => {
                Some(cache.get_compute_pipeline_descriptor(*pipeline_id))
            }
        }
    }

    pub fn get_render_pipeline<'a>(
        &'a self,
        cache: &'a PipelineCache,
        id: RenderResourceId,
    ) -> Option<&'a RenderPipeline> {
        let meta = self.render_pipelines.get(&id)?;
        match meta {
            RenderPipelineMeta::Direct(_, pipeline) => Some(pipeline.borrow()),
            RenderPipelineMeta::Cached(pipeline_id) => cache.get_render_pipeline(*pipeline_id),
        }
    }

    pub fn get_compute_pipeline<'a>(
        &'a self,
        cache: &'a PipelineCache,
        id: RenderResourceId,
    ) -> Option<&'a ComputePipeline> {
        let meta = self.compute_pipelines.get(&id)?;
        match meta {
            ComputePipelineMeta::Direct(_, pipeline) => Some(pipeline.borrow()),
            ComputePipelineMeta::Cached(pipeline_id) => cache.get_compute_pipeline(*pipeline_id),
        }
    }
}

impl RenderResource for RenderPipeline {
    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl DescribedRenderResource for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;

    #[inline]
    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        todo!()
    }
}

impl<'g> IntoRenderResource<'g> for RenderPipelineDescriptor {
    type Resource = RenderPipeline;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(self))
    }
}

impl RenderResource for ComputePipeline {
    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        todo!()
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        todo!()
    }
}

impl DescribedRenderResource for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;

    #[inline]
    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_compute_pipeline_direct(Some(descriptor), resource)
    }

    #[inline]
    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_compute_pipeline_descriptor(resource)
    }
}

impl<'g> IntoRenderResource<'g> for ComputePipelineDescriptor {
    type Resource = ComputePipeline;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_compute_pipeline_descriptor(self)
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
        let layout = graph.world_resource::<P>();
        let descriptor = layout.specialize(self.0);
        graph.new_resource(descriptor)
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
        let layout = graph.world_resource::<P>();
        let descriptor = layout.specialize(self.0);
        graph.new_resource(descriptor)
    }
}

pub struct SpecializeMeshPipeline<P: SpecializedMeshPipeline + Resource>(
    pub P::Key,
    pub MeshVertexBufferLayoutRef,
);

impl<'g, P: SpecializedMeshPipeline + Resource> IntoRenderResource<'g>
    for SpecializeMeshPipeline<P>
{
    type Resource = RenderPipeline;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        let layout = graph.world_resource::<P>();
        let descriptor = layout.specialize(self.0, &self.1).unwrap();
        graph.new_resource(descriptor)
    }
}
