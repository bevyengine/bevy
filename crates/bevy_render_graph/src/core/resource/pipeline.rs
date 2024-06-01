use std::borrow::{Borrow, Cow};

use bevy_asset::Handle;
use bevy_ecs::world::World;
use bevy_utils::HashMap;

use bevy_render::{
    prelude::Shader,
    render_resource::{
        BindGroupLayout, CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, DepthStencilState, FragmentState, MultisampleState,
        PipelineCache, PrimitiveState, PushConstantRange, RenderPipeline, RenderPipelineDescriptor,
        ShaderDefVal, VertexState,
    },
};

use crate::core::{Label, NodeContext, RenderGraph, RenderGraphBuilder};

use super::{
    IntoRenderResource, RenderDependencies, RenderHandle, RenderResource, RenderResourceId,
    ResourceTracker, ResourceType,
};

#[derive(Default)]
pub(in crate::core) struct CachedRenderGraphPipelines {
    cached_render_pipelines: HashMap<RenderPipelineDescriptor, CachedRenderPipelineId>,
    cached_compute_pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
}

#[derive(Default)]
pub(in crate::core) struct RenderGraphPipelines<'g> {
    render_pipelines: HashMap<RenderResourceId, RenderPipelineMeta<'g>>,
    existing_render_pipelines: HashMap<Cow<'g, RenderPipeline>, RenderResourceId>,
    queued_render_pipelines: HashMap<
        RenderResourceId,
        (
            RenderDependencies<'g>,
            RenderGraphRenderPipelineDescriptor<'g>,
        ),
    >,
    compute_pipelines: HashMap<RenderResourceId, ComputePipelineMeta<'g>>,
    existing_compute_pipelines: HashMap<Cow<'g, ComputePipeline>, RenderResourceId>,
    queued_compute_pipelines: HashMap<
        RenderResourceId,
        (
            RenderDependencies<'g>,
            RenderGraphComputePipelineDescriptor<'g>,
        ),
    >,
}

/// Describes a render pipeline in the render graph.
#[derive(Clone, PartialEq, Eq)]
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

impl<'g> RenderGraphRenderPipelineDescriptor<'g> {
    fn into_raw<'n>(self, context: &NodeContext<'n, 'g>) -> RenderPipelineDescriptor {
        let Self {
            label,
            layout,
            push_constant_ranges,
            vertex,
            primitive,
            depth_stencil,
            multisample,
            fragment,
        } = self;
        RenderPipelineDescriptor {
            label,
            layout: layout
                .into_iter()
                .map(|handle| context.get(handle).clone())
                .collect(),
            push_constant_ranges,
            vertex,
            primitive,
            depth_stencil,
            multisample,
            fragment,
        }
    }
}

/// Describes a compute pipeline in the render graph.
#[derive(Clone, PartialEq, Eq)]
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

impl<'g> RenderGraphComputePipelineDescriptor<'g> {
    fn into_raw<'n>(self, context: &NodeContext<'n, 'g>) -> ComputePipelineDescriptor {
        let Self {
            label,
            layout,
            push_constant_ranges,
            shader,
            shader_defs,
            entry_point,
        } = self;
        ComputePipelineDescriptor {
            label,
            layout: layout
                .into_iter()
                .map(|layout| context.get(layout).clone())
                .collect(),
            push_constant_ranges,
            shader,
            shader_defs,
            entry_point,
        }
    }
}

enum DirectOrCached<'g, D: Clone, C> {
    Direct(Cow<'g, D>),
    Cached(C),
}

struct RenderPipelineMeta<'g> {
    meta: RenderGraphRenderPipelineDescriptor<'g>,
    pipeline: DirectOrCached<'g, RenderPipeline, CachedRenderPipelineId>,
}

struct ComputePipelineMeta<'g> {
    meta: RenderGraphComputePipelineDescriptor<'g>,
    pipeline: DirectOrCached<'g, ComputePipeline, CachedComputePipelineId>,
}

impl<'g> RenderGraphPipelines<'g> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn import_render_pipeline(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        descriptor: RenderGraphRenderPipelineDescriptor<'g>,
        pipeline: Cow<'g, RenderPipeline>,
    ) -> RenderResourceId {
        let mut dependencies = RenderDependencies::new();
        for layout in &descriptor.layout {
            dependencies.read(*layout);
        }

        self.existing_render_pipelines
            .get(&pipeline)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(
                    descriptor.label.clone(),
                    ResourceType::RenderPipeline,
                    Some(dependencies),
                ); //todo: add layout dependencies
                self.render_pipelines.insert(
                    id,
                    RenderPipelineMeta {
                        meta: descriptor,
                        pipeline: DirectOrCached::Direct(pipeline),
                    },
                );
                id
            })
    }

    pub fn import_compute_pipeline(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        descriptor: RenderGraphComputePipelineDescriptor<'g>,
        pipeline: Cow<'g, ComputePipeline>,
    ) -> RenderResourceId {
        self.existing_compute_pipelines
            .get(&pipeline)
            .copied()
            .unwrap_or_else(|| {
                let id = tracker.new_resource(
                    descriptor.label.clone(),
                    ResourceType::ComputePipeline,
                    None,
                );
                self.compute_pipelines.insert(
                    id,
                    ComputePipelineMeta {
                        meta: descriptor,
                        pipeline: DirectOrCached::Direct(pipeline),
                    },
                );
                id
            })
    }

    pub fn new_render_pipeline(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        descriptor: RenderGraphRenderPipelineDescriptor<'g>,
    ) -> RenderResourceId {
        let mut dependencies = RenderDependencies::new();
        for layout in &descriptor.layout {
            dependencies.read(*layout);
        }
        let id = tracker.new_resource(
            descriptor.label.clone(),
            ResourceType::RenderPipeline,
            Some(dependencies.clone()),
        );
        self.queued_render_pipelines
            .insert(id, (dependencies, descriptor));
        id
    }

    pub fn new_compute_pipeline(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        descriptor: RenderGraphComputePipelineDescriptor<'g>,
    ) -> RenderResourceId {
        let mut dependencies = RenderDependencies::new();
        for layout in &descriptor.layout {
            dependencies.read(*layout);
        }
        let id = tracker.new_resource(
            descriptor.label.clone(),
            ResourceType::ComputePipeline,
            Some(dependencies.clone()),
        );
        self.queued_compute_pipelines
            .insert(id, (dependencies, descriptor));
        id
    }

    pub(in crate::core) fn create_queued_pipelines(
        &mut self,
        graph: &RenderGraph<'g>,
        local_cache: &mut CachedRenderGraphPipelines,
        pipeline_cache: &mut PipelineCache,
        world: &World,
    ) {
        for (resource_id, (dependencies, descriptor)) in self.queued_render_pipelines.drain() {
            let ctx = NodeContext {
                graph,
                world,
                dependencies,
                pipeline_cache: None,
            };
            let raw_descriptor = descriptor.clone().into_raw(&ctx);
            let pipeline_id = local_cache
                .cached_render_pipelines
                .entry(raw_descriptor.clone())
                .or_insert_with(|| pipeline_cache.queue_render_pipeline(raw_descriptor));
            self.render_pipelines.insert(
                resource_id,
                RenderPipelineMeta {
                    meta: descriptor,
                    pipeline: DirectOrCached::Cached(*pipeline_id),
                },
            );
        }

        for (resource_id, (dependencies, descriptor)) in self.queued_compute_pipelines.drain() {
            let ctx = NodeContext {
                graph,
                world,
                dependencies,
                pipeline_cache: None,
            };
            let raw_descriptor = descriptor.clone().into_raw(&ctx);
            let pipeline_id = local_cache
                .cached_compute_pipelines
                .entry(raw_descriptor.clone())
                .or_insert_with(|| pipeline_cache.queue_compute_pipeline(raw_descriptor));
            self.compute_pipelines.insert(
                resource_id,
                ComputePipelineMeta {
                    meta: descriptor,
                    pipeline: DirectOrCached::Cached(*pipeline_id),
                },
            );
        }

        pipeline_cache.process_queue();
    }

    //Note: currently fails when creating pipelines by descriptor. Might be a footgun but idk when
    //getting a pipeline's descriptor is that important
    pub fn get_render_pipeline_meta(
        &self,
        id: RenderResourceId,
    ) -> Option<&RenderGraphRenderPipelineDescriptor<'g>> {
        let meta = &self.render_pipelines.get(&id)?.meta;
        Some(meta)
    }

    //Note: currently fails when creating pipelines by descriptor. Might be a footgun but idk when
    //getting a pipeline's descriptor is that important
    pub fn get_compute_pipeline_meta(
        &self,
        id: RenderResourceId,
    ) -> Option<&RenderGraphComputePipelineDescriptor<'g>> {
        let meta = &self.compute_pipelines.get(&id)?.meta;
        Some(meta)
    }

    pub fn get_render_pipeline<'a>(
        &'a self,
        cache: &'a PipelineCache,
        id: RenderResourceId,
    ) -> Option<&'a RenderPipeline> {
        let meta = self.render_pipelines.get(&id)?;
        match &meta.pipeline {
            DirectOrCached::Direct(pipeline) => Some(pipeline.borrow()),
            DirectOrCached::Cached(pipeline_id) => cache.get_render_pipeline(*pipeline_id),
        }
    }

    pub fn get_compute_pipeline<'a>(
        &'a self,
        cache: &'a PipelineCache,
        id: RenderResourceId,
    ) -> Option<&'a ComputePipeline> {
        let meta = self.compute_pipelines.get(&id)?;
        match &meta.pipeline {
            DirectOrCached::Direct(pipeline) => Some(pipeline.borrow()),
            DirectOrCached::Cached(pipeline_id) => cache.get_compute_pipeline(*pipeline_id),
        }
    }
}

impl RenderResource for RenderPipeline {
    const RESOURCE_TYPE: ResourceType = ResourceType::RenderPipeline;
    type Meta<'g> = RenderGraphRenderPipelineDescriptor<'g>;

    #[inline]
    fn import_resource<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: Self::Meta<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_render_pipeline(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_render_pipeline(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_render_pipeline_meta(resource)
    }

    #[inline]
    fn meta_label<'g>(meta: &Self::Meta<'g>) -> Label<'g> {
        meta.label.clone()
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphRenderPipelineDescriptor<'g> {
    type Resource = RenderPipeline;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_render_pipeline(self)
    }
}

impl RenderResource for ComputePipeline {
    const RESOURCE_TYPE: ResourceType = ResourceType::ComputePipeline;
    type Meta<'g> = RenderGraphComputePipelineDescriptor<'g>;

    #[inline]
    fn import_resource<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        meta: RenderGraphComputePipelineDescriptor<'g>,
        resource: Cow<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.import_compute_pipeline(meta, resource)
    }

    #[inline]
    fn get<'n, 'g: 'n>(
        context: &'n NodeContext<'n, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'n Self> {
        context.get_compute_pipeline(resource)
    }

    #[inline]
    fn get_meta<'a, 'b: 'a, 'g: 'b>(
        graph: &'a RenderGraphBuilder<'b, 'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Meta<'g>> {
        graph.get_compute_pipeline_meta(resource)
    }

    #[inline]
    fn meta_label<'g>(meta: &Self::Meta<'g>) -> Label<'g> {
        meta.label.clone()
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphComputePipelineDescriptor<'g> {
    type Resource = ComputePipeline;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'_, 'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_compute_pipeline(self)
    }
}
