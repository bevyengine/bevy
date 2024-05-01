pub mod configurator;
pub mod resource;

use crate::{
    render_resource::{
        BindGroup, BindGroupLayout, Buffer, ComputePipeline, RenderPipeline, Sampler, Texture,
        TextureView,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::Resource,
    world::{EntityRef, Ref, World},
};

use resource::{IntoRenderResource, RenderHandle, RenderResource, RenderResources};

use wgpu::CommandEncoder;

use self::resource::{
    ref_eq::RefEq, CachedResources, DescribedRenderResource, RenderDependencies,
    RenderResourceGeneration, RenderResourceId, ResourceTracker, UsagesRenderResource,
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
pub struct RenderGraphPersistentResources {
    bind_group_layouts: CachedResources<BindGroupLayout>,
    samplers: CachedResources<Sampler>,
    render_pipelines: CachedResources<RenderPipeline>,
    compute_pipelines: CachedResources<ComputePipeline>,
}

pub struct RenderGraph<'g> {
    resources: ResourceTracker<'g>,
    bind_group_layouts: RenderResources<'g, BindGroupLayout>,
    // bind_groups: RenderBindGroups,
    textures: RenderResources<'g, Texture>,
    // texture_views: SimpleRenderResourceStore<'g, TextureView>,
    samplers: RenderResources<'g, Sampler>,
    buffers: RenderResources<'g, Buffer>,
    render_pipelines: RenderResources<'g, RenderPipeline>,
    compute_pipelines: RenderResources<'g, ComputePipeline>,
    //TODO:: store node graph here
}

impl<'g> RenderGraph<'g> {
    fn new() -> Self {
        todo!()
    }

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

impl<'g> RenderGraph<'g> {}

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
        R::into_render_resource(resource, self)
    }

    pub fn import_resource<R: RenderResource>(&mut self, resource: &'g R) -> RenderHandle<'g, R> {
        self.new_resource(RefEq::Borrowed(resource))
    }

    pub fn get_descriptor_of<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> Option<&'g R::Descriptor> {
        R::get_descriptor(self.graph, resource)
    }

    pub fn descriptor_of<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> &'g R::Descriptor {
        self.get_descriptor_of(resource)
            .expect("No descriptor found for resource")
    }

    pub fn add_usages<R: UsagesRenderResource>(
        &mut self,
        resource: RenderHandle<'g, R>,
        usages: R::Usages,
    ) -> &mut Self {
        let desc = R::get_descriptor_mut(self.graph, resource);
        if let Some(desc) = desc {
            R::add_usages(desc, usages);
        } else {
            let has_usages = R::get_descriptor(self.graph, resource)
                .map(|desc| R::has_usages(desc, &usages))
                .unwrap_or(true); //if no descriptor available, defer to wgpu to detect incorrect usage
            if !has_usages {
                panic!(
                    "Descriptor for resource {:?} does not contain usages: {:?}",
                    resource, usages
                )
            }
        }
        self
    }

    pub fn add_node(
        &mut self,
        dependencies: RenderDependencies<'g>,
        node: impl FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut CommandEncoder) + 'g,
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

    fn node_context(&'g self) -> NodeContext<'g> {
        NodeContext {
            graph: self.graph,
            world: self.world,
            view_entity: self.view_entity,
        }
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

#[derive(Copy, Clone)]
pub struct NodeContext<'g> {
    graph: &'g RenderGraph<'g>,
    world: &'g World,
    view_entity: EntityRef<'g>,
}

impl<'g> NodeContext<'g> {
    pub fn get<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> &R {
        R::get_from_store(self, resource).expect("Unable to locate render graph resource")
    }

    pub fn get_descriptor<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> Option<&R::Descriptor> {
        R::get_descriptor(self.graph, resource)
    }

    pub fn descriptor<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> &R::Descriptor {
        self.get_descriptor(resource)
            .expect("Resource does not have an associated descriptor")
    }

    fn get_texture(&self, texture: RenderHandle<'g, Texture>) -> Option<&Texture> {
        todo!()
    }

    fn get_texture_view(
        &self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> Option<&TextureView> {
        todo!()
    }

    fn get_sampler(&self, sampler: RenderHandle<'g, Sampler>) -> Option<&Sampler> {
        todo!()
    }

    fn get_buffer(&self, buffer: RenderHandle<'g, Buffer>) -> Option<&Buffer> {
        todo!()
    }

    fn get_render_pipeline(
        &self,
        render_pipeline: RenderHandle<'g, RenderPipeline>,
    ) -> Option<&RenderPipeline> {
        todo!()
    }

    fn get_compute_pipeline(
        &self,
        compute_pipeline: RenderHandle<'g, ComputePipeline>,
    ) -> Option<&ComputePipeline> {
        todo!()
    }

    fn get_bind_group_layout(
        &self,
        bind_group_layout: RenderHandle<'g, BindGroupLayout>,
    ) -> Option<&BindGroupLayout> {
        todo!()
    }

    fn get_bind_group(&self, bind_group: RenderHandle<'g, BindGroup>) -> Option<&BindGroup> {
        todo!()
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
