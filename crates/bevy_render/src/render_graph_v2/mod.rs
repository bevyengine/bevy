//mod build;
//mod compute_pass;
pub mod configurator;
pub mod resource;

mod seal {
    pub trait Super: Sized + Send + Sync + 'static {}
    pub struct Token;
}

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

use self::resource::{DependencySet, RenderRef, RenderResourceId, ResourceTracker};

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
    resources: ResourceTracker,
    bind_group_layouts: CachedRenderStore<BindGroupLayout>,
    // bind_groups: RenderBindGroups,
    textures: SimpleRenderStore<Texture>,
    views: SimpleRenderStore<TextureView>,
    samplers: CachedRenderStore<Sampler>,
    buffers: SimpleRenderStore<Buffer>,
    render_pipelines: CachedRenderStore<RenderPipeline>,
    compute_pipelines: CachedRenderStore<ComputePipeline>,
}

type RenderResourceGeneration = u16;

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

    fn new_resource_id(&mut self) -> RenderResourceId {
        self.resources.new_resource()
    }

    fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.resources[id]
    }

    fn write_resource<R: RenderResource>(&mut self, resource: RenderHandle<R>) -> &mut Self {
        self.resources.write(resource.id());
        self
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
        let id = self.graph.new_resource_id();
        <R::Resource as RenderResource>::get_store_mut(self.graph, seal::Token).insert(
            id,
            resource.into_render_resource(self.world, self.render_device),
            self.world,
            self.render_device,
        );
        RenderHandle::new(id)
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
        dependencies: impl Into<DependencySet>,
        node: impl FnOnce(NodeContext, &RenderDevice, &RenderQueue) + Send + Sync + 'a,
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

pub struct NodeContext<'a> {
    graph: &'a RenderGraph,
    world: &'a World,
    view_entity: EntityRef<'a>,
}

impl<'a> NodeContext<'a> {
    pub fn get<R: RenderResource>(&self, resource: RenderRef<R>) -> &'a R {
        todo!()
    }
}

impl<'a> NodeContext<'a> {
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
