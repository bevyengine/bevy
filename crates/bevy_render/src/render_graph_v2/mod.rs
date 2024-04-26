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
        BindGroupLayout, Buffer, ComputePipeline, PipelineCache, RenderPipeline, Sampler, Texture,
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

use resource::{IntoRenderResource, RenderHandle, RenderResource, RenderStore, SimpleRenderStore};

use resource::CachedRenderStore;

use self::resource::{
    IntoRenderDependencies, RenderDependencies, RenderResourceGeneration, RenderResourceId,
    RenderResourceInit, RenderResourceMeta, ResourceTracker,
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
struct RenderGraphPersistentResources {}

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
    //TODO:
}

impl<'g> RenderGraph<'g> {
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

    fn new_resource_id(&mut self) -> RenderResourceId {
        // self.resources.new_resource()
        todo!()
    }

    fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.resources.generation(id)
    }

    fn write_resource<R: RenderResource>(&mut self, resource: RenderHandle<R>) -> &mut Self {
        self.resources.write(resource.id());
        self
    }
}

// pub fn run_render_graph(
//     mut render_graph: ResMut<RenderGraph>,
//     render_device: &RenderDevice,
//     render_queue: &RenderQueue,
//     pipeline_cache: Res<PipelineCache>,
// ) {
//     render_graph.reset();
//     //render_graph.build(render_device, &pipeline_cache);
//     render_graph.run(render_device, render_queue);
// }

pub struct RenderGraphBuilder<'g> {
    graph: &'g mut RenderGraph<'g>,
    world: &'g World,
    view_entity: EntityRef<'g>,
    render_device: &'g RenderDevice,
}

impl<'g> RenderGraphBuilder<'g> {
    pub fn new_resource<R: IntoRenderResource<'g>>(
        &mut self,
        resource: R,
    ) -> RenderHandle<'g, R::Resource> {
        let id = self.graph.new_resource_id();
        <R::Resource as RenderResource>::get_store_mut(self.graph, seal::Token).insert(
            id,
            resource.into_render_resource(self.world, self.render_device),
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

    // pub fn import_resource<R: RenderResource>(
    //     &mut self,
    //     descriptor: Option<R::Descriptor>,
    //     resource: R::Data,
    // ) -> RenderHandle<R> {
    //     let next_id: u16 = self.graph.next_id;
    //     R::get_store_mut(self.graph, seal::Token).insert(
    //         next_id,
    //         RenderResourceInit::Eager(RenderResourceMeta {
    //             descriptor,
    //             resource,
    //         }),
    //         self.world,
    //         self.render_device,
    //     );
    //     self.graph.next_id += 1;
    //     RenderHandle::new(next_id)
    // }

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

    // pub fn new_resource_view<R: IntoRenderResourceView>(&mut self, )

    // pub fn retain<R: RetainedRenderResource>(
    //     &mut self,
    //     label: InternedRenderLabel,
    //     resource: RenderHandle<R>,
    // ) where
    //     R::Store: RetainedRenderStore<R>,
    // {
    //     R::get_store_mut(self.graph, seal::Token).retain(resource.index(), label);
    // }
    //
    // pub fn get_retained<R: RetainedRenderResource>(
    //     &mut self,
    //     label: InternedRenderLabel,
    // ) -> Option<RenderHandle<R>>
    // where
    //     R::Store: RetainedRenderStore<R>,
    // {
    //     let next_id: u16 = self.graph.next_id;
    //     let store = R::get_store_mut(self.graph, seal::Token);
    //     let res = store.get_retained(label)?;
    //     store.insert(
    //         next_id,
    //         RenderResourceInit::Eager(res),
    //         self.world,
    //         self.render_device,
    //     );
    //     self.graph.next_id += 1;
    //     Some(RenderHandle::new(next_id))
    // }

    // pub fn new_bind_group<M, D: RenderData<M>, B: AsBindGroup>(
    //     &mut self,
    //     dependencies: impl IntoRenderData<M, Data = D>,
    //     node: impl FnOnce(NodeContext<'_, M, D>, &RenderDevice) -> B + Send + Sync + 'static,
    // ) -> RenderBindGroup {
    //     todo!()
    // }

    pub fn add_node(
        &mut self,
        dependencies: RenderDependencies<'g>,
        node: impl FnOnce(NodeContext, &RenderDevice, &RenderQueue) + Send + Sync + 'g,
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
    pub fn world_resource<R: Resource>(&'g self) -> &'g R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&'g self) -> Option<&'g R> {
        self.world.get_resource()
    }

    pub fn view_id(&self) -> Entity {
        self.view_entity.id()
    }

    pub fn view_contains<C: Component>(&'g self) -> bool {
        self.view_entity.contains::<C>()
    }

    pub fn view_get<C: Component>(&'g self) -> Option<&'g C> {
        self.view_entity.get()
    }

    pub fn view_get_ref<C: Component>(&'g self) -> Option<Ref<'g, C>> {
        self.view_entity.get_ref()
    }

    pub fn view_entity(&'g self) -> EntityRef<'g> {
        self.view_entity
    }

    pub fn world(&'g self) -> &'g World {
        self.world
    }
}
