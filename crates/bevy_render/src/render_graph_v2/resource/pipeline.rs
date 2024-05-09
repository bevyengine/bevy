use std::{
    borrow::{Borrow, Cow},
    mem,
};

use bevy_asset::Handle;
use bevy_ecs::system::Resource;
use bevy_utils::HashMap;
use wgpu::{DepthStencilState, MultisampleState, PrimitiveState, PushConstantRange};

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    prelude::Shader,
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{
        BindGroupLayout, CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, FragmentState, PipelineCache, RenderPipeline,
        RenderPipelineDescriptor, ShaderDefVal, SpecializedComputePipeline,
        SpecializedMeshPipeline, SpecializedRenderPipeline, VertexState,
    },
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, IntoRenderResource, NewRenderResource, RenderHandle,
    RenderResource, RenderResourceId, ResourceTracker, ResourceType,
};

#[derive(Default)]
pub struct CachedRenderGraphPipelines {
    cached_render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
    cached_compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
}

#[derive(Default)]
pub struct RenderGraphPipelines<'g> {
    render_pipelines: HashMap<RenderResourceId, RenderPipelineMeta<'g>>,
    existing_render_pipelines: HashMap<RefEq<'g, RenderPipeline>, RenderResourceId>,
    queued_render_pipelines: HashMap<RenderResourceId, RenderGraphRenderPipelineDescriptor<'g>>,
    compute_pipelines: HashMap<RenderResourceId, ComputePipelineMeta<'g>>,
    existing_compute_pipelines: HashMap<RefEq<'g, ComputePipeline>, RenderResourceId>,
    queued_compute_pipelines: HashMap<RenderResourceId, RenderGraphComputePipelineDescriptor<'g>>,
}

/// Describes a render pipeline in the render graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderGraphRenderPipelineDescriptor<'g> {
    /// Debug label of the pipeline. This will show up in graphics debuggers for easy identification.
    pub label: Option<Cow<'static, str>>,
    /// The layout of bind groups for this pipeline.
    pub layout: Vec<RenderHandle<'g, BindGroupLayout>>,
    /// The push constant ranges for this pipeline.
    /// Supply an empty vector if the pipeline doesn't use push constants.
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// The compiled vertex stage, its entry point, and the input buffers layout.
    pub vertex: VertexState,
    /// The properties of the pipeline at the primitive assembly and rasterization level.
    pub primitive: PrimitiveState,
    /// The effect of draw calls on the depth and stencil aspects of the output target, if any.
    pub depth_stencil: Option<DepthStencilState>,
    /// The multi-sampling properties of the pipeline.
    pub multisample: MultisampleState,
    /// The compiled fragment stage, its entry point, and the color targets.
    pub fragment: Option<FragmentState>,
}

/// Describes a compute pipeline in the render graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderGraphComputePipelineDescriptor<'g> {
    pub label: Option<Cow<'static, str>>,
    pub layout: Vec<RenderHandle<'g, BindGroupLayout>>,
    pub push_constant_ranges: Vec<PushConstantRange>,
    /// The compiled shader module for this stage.
    pub shader: Handle<Shader>,
    pub shader_defs: Vec<ShaderDefVal>,
    /// The name of the entry point in the compiled shader. There must be a
    /// function with this name in the shader.
    pub entry_point: Cow<'static, str>,
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
        self.existing_render_pipelines
            .get(&pipeline)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(ResourceType::RenderPipeline, None); //todo: add layout dependencies
                self.render_pipelines
                    .insert(id, RenderPipelineMeta::Direct(descriptor, pipeline));
                id
            })
    }

    pub fn new_direct_compute_pipeline(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: Option<ComputePipelineDescriptor>,
        pipeline: RefEq<'g, ComputePipeline>,
    ) -> RenderResourceId {
        self.existing_compute_pipelines
            .get(&pipeline)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(ResourceType::ComputePipeline, None);
                self.compute_pipelines
                    .insert(id, ComputePipelineMeta::Direct(descriptor, pipeline));
                id
            })
    }

    pub fn new_render_pipeline_descriptor(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: RenderGraphRenderPipelineDescriptor<'g>,
    ) -> RenderResourceId {
        let id = tracker.new_resource(ResourceType::RenderPipeline, None);
        self.queued_render_pipelines.insert(id, descriptor);
        id
    }

    pub fn new_compute_pipeline_descriptor(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: RenderGraphComputePipelineDescriptor<'g>,
    ) -> RenderResourceId {
        let id = tracker.new_resource(ResourceType::ComputePipeline, None);
        self.queued_compute_pipelines.insert(id, descriptor);
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
    const RESOURCE_TYPE: ResourceType = ResourceType::RenderPipeline;

    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_render_pipeline_direct(None, resource)
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_render_pipeline(resource)
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
        graph.new_render_pipeline_direct(Some(descriptor), resource)
    }

    #[inline]
    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_render_pipeline_descriptor(resource)
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphRenderPipelineDescriptor<'g> {
    type Resource = RenderPipeline;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_render_pipeline_descriptor(self)
    }
}

impl<'g> IntoRenderResource<'g> for RenderPipelineDescriptor {
    type Resource = RenderPipeline;

    fn into_render_resource(
        mut self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        let RenderPipelineDescriptor {
            label,
            layout,
            push_constant_ranges,
            vertex,
            primitive,
            depth_stencil,
            multisample,
            fragment,
        } = self;
        let descriptor = RenderGraphRenderPipelineDescriptor {
            label,
            layout: layout
                .into_iter()
                .map(|layout| graph.into_resource(layout))
                .collect(),
            push_constant_ranges,
            vertex,
            primitive,
            depth_stencil,
            multisample,
            fragment,
        };
        graph.new_resource(descriptor)
    }
}

impl RenderResource for ComputePipeline {
    const RESOURCE_TYPE: ResourceType = ResourceType::ComputePipeline;

    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_compute_pipeline_direct(None, resource)
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_compute_pipeline(resource)
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
