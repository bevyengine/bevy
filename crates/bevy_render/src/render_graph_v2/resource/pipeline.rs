use bevy_ecs::{
    system::{Res, ResMut, Resource, SystemState},
    world::World,
};
use bevy_utils::HashMap;

use crate::{
    mesh::MeshVertexBufferLayoutRef,
    render_graph_v2::RenderGraph,
    render_resource::{
        CachedComputePipelineId, CachedRenderPipelineId, ComputePipeline,
        ComputePipelineDescriptor, PipelineCache, RenderPipeline, RenderPipelineDescriptor,
        SpecializedComputePipeline, SpecializedComputePipelines, SpecializedMeshPipeline,
        SpecializedMeshPipelines, SpecializedRenderPipeline, SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
};

use super::{
    DeferredResourceInit, IntoRenderResource, RenderResource, RenderResourceId, RenderResourceInit,
    RenderResourceMeta, RenderStore,
};

#[derive(Default)]
pub struct RenderGraphPipelines {
    render_pipelines: HashMap<u16, RenderResourceMeta<RenderPipeline>>,
    queued_render_pipelines: HashMap<u16, DeferredResourceInit<RenderPipeline>>,
    compute_pipelines: HashMap<u16, RenderResourceMeta<ComputePipeline>>,
    queued_compute_pipelines: HashMap<u16, DeferredResourceInit<ComputePipeline>>,
}

impl RenderStore<RenderPipeline> for RenderGraphPipelines {
    fn insert(&mut self, key: u16, data: RenderResourceInit<RenderPipeline>) {
        todo!()
    }

    fn get<'a>(
        &'a self,
        world: &'a World,
        key: u16,
    ) -> Option<&'a RenderResourceMeta<RenderPipeline>> {
        todo!()
    }

    fn init_queued_resources(&mut self, world: &mut World, device: &RenderDevice) {
        todo!()
    }
}

impl RenderStore<ComputePipeline> for RenderGraphPipelines {
    fn insert(&mut self, key: u16, data: RenderResourceInit<ComputePipeline>) {
        todo!()
    }

    fn get<'a>(
        &'a self,
        world: &'a World,
        key: u16,
    ) -> Option<&'a RenderResourceMeta<ComputePipeline>> {
        todo!()
    }

    fn init_queued_resources(&mut self, world: &mut World, device: &RenderDevice) {
        todo!()
    }
}

impl RenderGraphPipelines {
    pub fn insert_render_pipelines(
        &mut self,
        key: RenderResourceId,
        resource: RenderResourceInit<RenderPipeline>,
    ) {
        match resource {
            RenderResourceInit::Eager(meta) => {
                self.render_pipelines.insert(key.index, meta);
            }
            RenderResourceInit::Deferred(init) => {
                self.queued_render_pipelines.insert(key.index, init);
            }
        }
    }

    pub fn init_deferred(&mut self, world: &mut World, render_device: &RenderDevice) {
        for (id, init) in self.queued_render_pipelines.drain() {
            self.render_pipelines
                .insert(id, (init)(world, render_device));
        }

        for (id, init) in self.queued_compute_pipelines.drain() {
            self.compute_pipelines
                .insert(id, (init)(world, render_device));
        }
    }

    pub fn get_render_pipeline(
        &self,
        key: RenderResourceId,
    ) -> Option<&RenderResourceMeta<RenderPipeline>> {
        self.render_pipelines.get(&key.index)
    }

    pub fn get_compute_pipeline(
        &self,
        key: RenderResourceId,
    ) -> Option<&RenderResourceMeta<ComputePipeline>> {
        self.compute_pipelines.get(&key.index)
    }
}

impl RenderResource for RenderPipeline {
    type Descriptor = RenderPipelineDescriptor;
    type Data = CachedRenderPipelineId;
    type Store = RenderGraphPipelines;

    fn get_store(graph: &RenderGraph) -> &Self::Store {
        &graph.pipelines
    }

    fn get_store_mut(graph: &mut RenderGraph) -> &mut Self::Store {
        &mut graph.pipelines
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        world.resource::<PipelineCache>().get_render_pipeline(*data)
    }
}

impl RenderResource for ComputePipeline {
    type Descriptor = ComputePipelineDescriptor;
    type Data = CachedComputePipelineId;
    type Store = RenderGraphPipelines;

    fn get_store(graph: &RenderGraph) -> &Self::Store {
        &graph.pipelines
    }

    fn get_store_mut(graph: &mut RenderGraph) -> &mut Self::Store {
        &mut graph.pipelines
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        world
            .resource::<PipelineCache>()
            .get_compute_pipeline(*data)
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
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::Deferred(Box::new(|world, _| {
            world.init_resource::<SpecializedRenderPipelines<P>>();
            let (mut specializer, pipelines, layout) = SystemState::<(
                ResMut<SpecializedRenderPipelines<P>>,
                Res<PipelineCache>,
                Res<P>,
            )>::new(world)
            .get_mut(world);

            let pipeline = specializer.specialize(&pipelines, &layout, self.0);
            RenderResourceMeta {
                descriptor: None,
                resource: pipeline,
            }
        }))
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
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::Deferred(Box::new(move |world, _| {
            world.init_resource::<SpecializedComputePipelines<P>>();
            let (mut specializer, pipelines, layout) = SystemState::<(
                ResMut<SpecializedComputePipelines<P>>,
                Res<PipelineCache>,
                Res<P>,
            )>::new(world)
            .get_mut(world);

            let pipeline = specializer.specialize(&pipelines, &layout, self.0);
            RenderResourceMeta {
                descriptor: None,
                resource: pipeline,
            }
        }))
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
        _world: &World,
        _render_device: &RenderDevice,
    ) -> RenderResourceInit<Self::Resource> {
        RenderResourceInit::Deferred(Box::new(move |world, _| {
            world.init_resource::<SpecializedMeshPipelines<P>>();
            let (mut specializer, pipelines, layout) = SystemState::<(
                ResMut<SpecializedMeshPipelines<P>>,
                Res<PipelineCache>,
                Res<P>,
            )>::new(world)
            .get_mut(world);

            let pipeline = specializer
                .specialize(&pipelines, &layout, self.1, &self.0)
                .expect("Unable to specialize mesh pipeline.");
            RenderResourceMeta {
                descriptor: None,
                resource: pipeline,
            }
        }))
    }
}
