//mod build;
//mod compute_pass;
pub mod configurator;
pub mod resource;

use std::{any::TypeId, marker::PhantomData};

use crate::{
    render_graph::InternedRenderLabel,
    render_resource::{
        BindGroup, Buffer, ComputePipeline, PipelineCache, RenderPipeline, Sampler, Texture,
        TextureView,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Res, ResMut, Resource},
    world::{EntityRef, Ref, World},
};
use resource::bind_group::{AsRenderBindGroup, RenderBindGroup};

use resource::{
    bind_group::RenderBindGroups, DependencySet, IntoRenderResource, RenderHandle, RenderResource,
    RenderResourceInit, RenderStore, RetainedRenderResource, RetainedRenderStore,
    SimpleRenderStore,
};

use resource::CachedRenderStore;

use self::resource::{IntoRenderData, RenderData, RenderResourceMeta};

// Roadmap:
// 1. Autobuild (and cache) bind group layouts, textures, bind groups, and compute pipelines
// 2. Run the graph in the correct order (figure out how the API should handle command encoders/buffers)
// 3. Buffer and sampler support
// 4. Allow importing external textures
// 5. Temporal resources
// 6. Start porting the engine as a proof of concept/demo, and fill in missing features (e.g. raster nodes)
// 7. Auto-insert CPU profiling, GPU profiling, and GPU debug markers (probably need some concept of a group of render nodes)
// 8. Documentation, write an example, and cleanup

#[derive(Resource, Default)]
pub struct RenderGraph {
    // TODO: maybe use a Vec for resource_descriptors, and replace next_id with resource_descriptors.len()
    next_id: u16, //resource_descriptors: HashMap<RenderGraphResourceId, TextureDescriptor<'static>>,
    // nodes: Vec<RenderGraphNode>,
    //
    // bind_group_layouts: HashMap<Box<[BindGroupLayoutEntry]>, BindGroupLayout>,
    // resources: HashMap<RenderGraphResourceId, Texture>,
    // pipelines: HashMap<ComputePipelineDescriptor, CachedComputePipelineId>,
    bind_groups: RenderBindGroups,
    textures: SimpleRenderStore<Texture>,
    views: SimpleRenderStore<TextureView>,
    samplers: CachedRenderStore<Sampler>,
    buffers: SimpleRenderStore<Buffer>,
    render_pipelines: CachedRenderStore<RenderPipeline>,
    compute_pipelines: CachedRenderStore<ComputePipeline>,
}

impl RenderGraph {
    pub(crate) fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // TODO
    }

    pub(crate) fn reset(&mut self) {
        // self.next_id = 0;
        // self.resource_descriptors.clear();
        // self.nodes.clear();
        //
        // TODO: Remove unused resources
    }
}

pub fn run_render_graph(
    mut render_graph: ResMut<RenderGraph>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    pipeline_cache: Res<PipelineCache>,
) {
    render_graph.reset();
    //render_graph.build(render_device, &pipeline_cache);
    render_graph.run(render_device, render_queue);
}

pub struct RenderGraphBuilder<'a> {
    graph: &'a mut RenderGraph,
    world: &'a World,
    view_entity: EntityRef<'a>,
    render_device: &'a RenderDevice,
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn new_resource<R: IntoRenderResource>(
        &mut self,
        resource: R,
    ) -> RenderHandle<R::Resource> {
        let next_id: u16 = self.graph.next_id;
        <R::Resource as RenderResource>::get_store_mut(self.graph).insert(
            next_id,
            resource.into_render_resource(self.world, self.render_device),
            self.world,
            self.render_device,
        );
        self.graph.next_id += 1;
        RenderHandle::new(next_id)
    }

    pub fn import_resource<R: RenderResource>(
        &mut self,
        descriptor: Option<R::Descriptor>,
        resource: R::Data,
    ) -> RenderHandle<R> {
        let next_id: u16 = self.graph.next_id;
        R::get_store_mut(self.graph).insert(
            next_id,
            RenderResourceInit::Eager(RenderResourceMeta {
                descriptor,
                resource,
            }),
            self.world,
            self.render_device,
        );
        self.graph.next_id += 1;
        RenderHandle::new(next_id)
    }

    pub fn get_descriptor_of<R: RenderResource>(
        &self,
        resource: RenderHandle<R>,
    ) -> Option<&R::Descriptor> {
        R::get_store(self.graph)
            .get(self.world, resource.index())
            .and_then(|meta| meta.descriptor.as_ref())
    }

    pub fn descriptor_of<R: RenderResource>(&self, resource: RenderHandle<R>) -> &R::Descriptor {
        self.get_descriptor_of(resource)
            .expect("No descriptor found for resource")
    }

    pub fn retain<R: RetainedRenderResource>(
        &mut self,
        label: InternedRenderLabel,
        resource: RenderHandle<R>,
    ) where
        R::Store: RetainedRenderStore<R>,
    {
        R::get_store_mut(self.graph).retain(resource.index(), label);
    }

    pub fn get_retained<R: RetainedRenderResource>(
        &mut self,
        label: InternedRenderLabel,
    ) -> Option<RenderHandle<R>>
    where
        R::Store: RetainedRenderStore<R>,
    {
        let next_id: u16 = self.graph.next_id;
        let store = R::get_store_mut(self.graph);
        let res = store.get_retained(label)?;
        store.insert(
            next_id,
            RenderResourceInit::Eager(res),
            self.world,
            self.render_device,
        );
        self.graph.next_id += 1;
        Some(RenderHandle::new(next_id))
    }

    pub fn new_bind_group<B: AsRenderBindGroup>(&mut self, desc: B) -> RenderBindGroup {
        todo!()
    }

    pub fn add_node<
        'n,
        D: RenderData,
        F: FnOnce(NodeContext<'_, D>, &RenderDevice, &RenderQueue) + 'static,
    >(
        &'n mut self,
        dependencies: impl IntoRenderData<Data = D>,
        node: F,
    ) -> &mut Self {
        todo!();
        self
    }

    pub fn features(&self) -> wgpu::Features {
        self.render_device.features()
    }

    pub fn limits(&self) -> wgpu::Limits {
        self.render_device.limits()
    }
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn world_resource<R: Resource>(&'a self) -> &'a R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&'a self) -> Option<&'a R> {
        self.world.get_resource()
    }

    pub fn view_id(&self) -> Entity {
        self.view_entity.id()
    }

    pub fn view_contains<C: Component>(&'a self) -> bool {
        self.view_entity.contains::<C>()
    }

    pub fn view_get<C: Component>(&'a self) -> Option<&'a C> {
        self.view_entity.get()
    }

    pub fn view_get_ref<C: Component>(&'a self) -> Option<Ref<'a, C>> {
        self.view_entity.get_ref()
    }

    pub fn view_entity(&'a self) -> EntityRef<'a> {
        self.view_entity
    }

    pub fn world(&'a self) -> &'a World {
        self.world
    }
}

pub struct NodeContext<'a, D: RenderData> {
    graph: &'a mut RenderGraph,
    world: &'a World,
    view_entity: EntityRef<'a>,
    input: D::Handle,
}

impl<'a, D: RenderData> NodeContext<'a, D> {
    pub fn input(&mut self) -> D::Item<'_> {
        D::get_from_graph(&self.input, self.graph, self.world)
            .expect("Could not access input for node. NodeContext::input() is not guaranteed to succeed when called more than once")
    }
}

impl<'a, D: RenderData> NodeContext<'a, D> {
    pub fn world_resource<R: Resource>(&'a self) -> &'a R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&'a self) -> Option<&'a R> {
        self.world.get_resource()
    }

    pub fn view_id(&self) -> Entity {
        self.view_entity.id()
    }

    pub fn view_contains<C: Component>(&'a self) -> bool {
        self.view_entity.contains::<C>()
    }

    pub fn view_get<C: Component>(&'a self) -> Option<&'a C> {
        self.view_entity.get()
    }

    pub fn view_get_ref<C: Component>(&'a self) -> Option<Ref<'a, C>> {
        self.view_entity.get_ref()
    }

    pub fn view_entity(&'a self) -> EntityRef<'a> {
        self.view_entity
    }

    pub fn world(&'a self) -> &'a World {
        self.world
    }
}
