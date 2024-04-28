//mod build;
//mod compute_pass;
pub mod configurator;
pub mod resource;

mod seal {
    pub trait Super: Sized + Send + Sync {}
    pub struct Token;
}

use std::borrow::Cow;

use crate::{
    render_resource::{
        BindGroupLayout, Buffer, ComputePipeline, RenderPipeline, Sampler, Texture, TextureView,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::Resource,
    world::{EntityRef, Ref, World},
};

use resource::{IntoRenderResource, RenderHandle, RenderResource, RenderStore, SimpleRenderStore};

use resource::CachedRenderStore;
use wgpu::CommandEncoder;

use self::resource::{
    CachedRenderStorePersistentResources, RenderDependencies, RenderResourceGeneration,
    RenderResourceId, RenderResourceInit, RenderResourceMeta, ResourceTracker,
};

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
struct RenderGraphPersistentResources {
    dummy: (),
    bind_group_layouts: CachedRenderStorePersistentResources<BindGroupLayout>,
    samplers: CachedRenderStorePersistentResources<Sampler>,
    render_pipelines: CachedRenderStorePersistentResources<RenderPipeline>,
    compute_pipelines: CachedRenderStorePersistentResources<ComputePipeline>,
}

#[derive(Default)]
pub struct RenderGraph<'g> {
    resources: ResourceTracker<'g>,
    bind_group_layouts: CachedRenderStore<'g, BindGroupLayout>,
    // bind_groups: RenderBindGroups,
    textures: SimpleRenderStore<'g, Texture>,
    views: SimpleRenderStore<'g, TextureView>,
    samplers: CachedRenderStore<'g, Sampler>,
    buffers: SimpleRenderStore<'g, Buffer>,
    render_pipelines: CachedRenderStore<'g, RenderPipeline>,
    compute_pipelines: CachedRenderStore<'g, ComputePipeline>,
    //TODO:: store node graph here
}

impl<'g> RenderGraph<'g> {
    fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // TODO
    }

    fn new_resource_id(
        &mut self,
        dependencies: Option<RenderDependencies<'g>>,
    ) -> RenderResourceId {
        self.resources.new_resource(dependencies)
    }

    fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.resources.generation(id)
    }

    fn write_resources(&mut self, dependencies: &RenderDependencies<'g>) -> &mut Self {
        dependencies
            .iter_writes()
            .for_each(|id| self.resources.write(id));
        self
    }
}

pub struct RenderGraphBuilder<'g> {
    graph: &'g mut RenderGraph<'g>,
    persistent_resources: &'g mut RenderGraphPersistentResources,
    world: &'g World,
    view_entity: EntityRef<'g>,
    render_device: &'g RenderDevice,
}

impl<'g> RenderGraphBuilder<'g> {
    pub fn new_resource<R: IntoRenderResource<'g>>(
        &mut self,
        resource: R,
    ) -> RenderHandle<'g, R::Resource> {
        let resource = resource.into_render_resource(self.world, self.render_device);
        let id = self.graph.new_resource_id(match &resource {
            RenderResourceInit::DependentResource(deps, _, _) => Some(deps.clone()),
            _ => None,
        });
        <R::Resource as RenderResource>::get_store_mut(self.graph, seal::Token).insert(
            <R::Resource as RenderResource>::get_persistent_store_mut(
                &mut self.persistent_resources,
                seal::Token,
            ),
            id,
            resource,
            self.world,
            self.render_device,
        );
        RenderHandle::new(id)
    }

    pub fn import_resource<R: RenderResource>(
        &mut self,
        descriptor: Option<R::Descriptor>,
        resource: &'g R::Data,
    ) -> RenderHandle<'g, R> {
        self.new_resource(RenderResourceMeta {
            descriptor,
            resource: Cow::Borrowed(resource),
        })
    }

    pub fn get_descriptor_of<R: RenderResource>(
        &self,
        resource: RenderHandle<R>,
    ) -> Option<&R::Descriptor> {
        R::get_store(self.graph, seal::Token)
            .get(self.world, resource.id())
            .and_then(|meta| meta.descriptor.as_ref())
    }

    pub fn descriptor_of<R: RenderResource>(&self, resource: RenderHandle<R>) -> &R::Descriptor {
        self.get_descriptor_of(resource)
            .expect("No descriptor found for resource")
    }

    pub fn add_node(
        &mut self,
        dependencies: RenderDependencies<'g>,
        node: impl FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut CommandEncoder)
            + Send
            + Sync
            + 'g,
    ) -> &mut Self {
        self.graph.write_resources(&dependencies);
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

pub struct NodeContext<'g> {
    graph: &'g RenderGraph<'g>,
    world: &'g World,
    view_entity: EntityRef<'g>,
}

impl<'g> NodeContext<'g> {
    pub fn get<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> &'g R {
        R::get_store(self.graph, seal::Token)
            .get(self.world, resource.id())
            .and_then(|meta| R::from_data(&meta.resource, self.world))
            .expect("Unable to locate render graph resource")
    }
}

impl<'g> NodeContext<'g> {
    pub fn world_resource<R: Resource>(&self) -> &R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&self) -> Option<&'g R> {
        self.world.get_resource()
    }

    pub fn view_id(&self) -> Entity {
        self.view_entity.id()
    }

    pub fn view_contains<C: Component>(&self) -> bool {
        self.view_entity.contains::<C>()
    }

    pub fn view_get<C: Component>(&self) -> Option<&'g C> {
        self.view_entity.get()
    }

    pub fn view_get_ref<C: Component>(&self) -> Option<Ref<'g, C>> {
        self.view_entity.get_ref()
    }

    pub fn view_entity(&'g self) -> EntityRef<'g> {
        self.view_entity
    }

    pub fn world(&'g self) -> &'g World {
        self.world
    }
}
