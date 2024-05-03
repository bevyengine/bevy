pub mod configurator;
pub mod resource;

use crate::{
    render_resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutEntries, Buffer, ComputePipeline,
        ComputePipelineDescriptor, PipelineCache, RenderPipeline, RenderPipelineDescriptor,
        Sampler, Texture, TextureView,
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

use wgpu::{
    BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BufferDescriptor,
    CommandEncoder, SamplerDescriptor, TextureDescriptor, TextureViewDescriptor,
};

use self::resource::{
    pipeline::{CachedRenderGraphPipelines, RenderGraphPipelines},
    ref_eq::RefEq,
    texture::RenderGraphSamplerDescriptor,
    CachedResources, DescribedRenderResource, RenderDependencies, RenderResourceGeneration,
    RenderResourceId, ResourceTracker, UsagesRenderResource,
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
    pipelines: CachedRenderGraphPipelines,
}

pub struct RenderGraph<'g> {
    resources: ResourceTracker<'g>,
    bind_group_layouts: RenderResources<'g, BindGroupLayout>,
    // bind_groups: RenderBindGroups,
    textures: RenderResources<'g, Texture>,
    // texture_views: SimpleRenderResourceStore<'g, TextureView>,
    samplers: RenderResources<'g, Sampler>,
    buffers: RenderResources<'g, Buffer>,
    pipelines: RenderGraphPipelines<'g>,
    //TODO:: store node graph here
}

impl<'g> RenderGraph<'g> {
    fn new() -> Self {
        Self {
            resources: ResourceTracker::default(),
            bind_group_layouts: RenderResources::new(
                |device, desc: &Box<[BindGroupLayoutEntry]>| {
                    device.create_bind_group_layout(None, &desc)
                },
            ),
            textures: RenderResources::new(RenderDevice::create_texture),
            samplers: RenderResources::new(|device, RenderGraphSamplerDescriptor(desc)| {
                device.create_sampler(desc)
            }),
            buffers: RenderResources::new(RenderDevice::create_buffer),
            pipelines: RenderGraphPipelines::new(),
        }
    }

    fn run(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        // TODO
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

    fn create_queued_resources(
        &mut self,
        cache: &'g mut RenderGraphPersistentResources,
        render_device: &RenderDevice,
    ) {
        self.bind_group_layouts
            .create_queued_resources_cached(&mut cache.bind_group_layouts, render_device);
        //pipelines here
        self.textures.create_queued_resources(render_device);
        //texture views here
        self.samplers
            .create_queued_resources_cached(&mut cache.samplers, render_device);
        self.buffers.create_queued_resources(render_device);
        //bind groups here
    }
}

impl<'g> RenderGraph<'g> {
    #[inline]
    fn get_bind_group_layout_descriptor(
        &self,
        bind_group_layout: RenderHandle<'g, BindGroupLayout>,
    ) -> Option<&Box<[BindGroupLayoutEntry]>> {
        self.bind_group_layouts
            .get_descriptor(bind_group_layout.id())
    }

    #[inline]
    fn get_texture_descriptor(
        &self,
        texture: RenderHandle<'g, Texture>,
    ) -> Option<&TextureDescriptor<'static>> {
        self.textures.get_descriptor(texture.id())
    }

    #[inline]
    fn get_texture_descriptor_mut(
        &mut self,
        texture: RenderHandle<'g, Texture>,
    ) -> Option<&mut TextureDescriptor<'static>> {
        self.textures.get_descriptor_mut(texture.id())
    }

    #[inline]
    fn get_texture_view_descriptor(
        &self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> Option<&TextureViewDescriptor<'static>> {
        todo!()
        // self.texture_views.get_descriptor(texture_view.id())
    }

    #[inline]
    fn get_sampler_descriptor(
        &self,
        sampler: RenderHandle<'g, Sampler>,
    ) -> Option<&RenderGraphSamplerDescriptor> {
        self.samplers.get_descriptor(sampler.id())
    }

    #[inline]
    fn get_buffer_descriptor(
        &self,
        buffer: RenderHandle<'g, Buffer>,
    ) -> Option<&BufferDescriptor<'static>> {
        self.buffers.get_descriptor(buffer.id())
    }

    #[inline]
    fn get_buffer_descriptor_mut(
        &mut self,
        buffer: RenderHandle<'g, Buffer>,
    ) -> Option<&mut BufferDescriptor<'static>> {
        self.buffers.get_descriptor_mut(buffer.id())
    }

    #[inline]
    fn get_render_pipeline_descriptor<'a>(
        &'a self,
        render_pipeline: RenderHandle<'g, RenderPipeline>,
        pipeline_cache: &'a PipelineCache,
    ) -> Option<&'a RenderPipelineDescriptor>
    where
        'g: 'a,
    {
        self.pipelines
            .get_render_pipeline_descriptor(pipeline_cache, render_pipeline.id())
    }

    #[inline]
    fn get_compute_pipeline_descriptor<'a>(
        &'a self,
        compute_pipeline: RenderHandle<'g, ComputePipeline>,
        pipeline_cache: &'a PipelineCache,
    ) -> Option<&'a ComputePipelineDescriptor>
    where
        'g: 'a,
    {
        self.pipelines
            .get_compute_pipeline_descriptor(pipeline_cache, compute_pipeline.id())
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
        R::into_render_resource(resource, self)
    }

    pub fn import_resource<R: RenderResource>(&mut self, resource: &'g R) -> RenderHandle<'g, R> {
        self.new_resource(RefEq::Borrowed(resource))
    }

    pub fn get_descriptor_of<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> Option<&R::Descriptor> {
        R::get_descriptor(self.graph, resource)
    }

    pub fn descriptor_of<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> &R::Descriptor {
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
                    "Descriptor for resource {:?} does not contain necessary usages: {:?}",
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

impl<'g> RenderGraphBuilder<'g> {
    #[inline]
    fn new_bind_group_layout_direct(
        &mut self,
        descriptor: Option<Box<[BindGroupLayoutEntry]>>,
        bind_group_layout: RefEq<'g, BindGroupLayout>,
    ) -> RenderHandle<'g, BindGroupLayout> {
        todo!()
    }

    #[inline]
    fn new_bind_group_layout_descriptor(
        &mut self,
        descriptor: Box<[BindGroupLayoutEntry]>,
    ) -> RenderHandle<'g, BindGroupLayout> {
        todo!();
    }

    #[inline]
    fn new_bind_group_direct(
        &mut self,
        dependencies: RenderDependencies<'g>,
        bind_group: RefEq<'g, BindGroup>,
    ) -> RenderHandle<'g, BindGroup> {
        todo!()
    }

    #[inline]
    fn new_bind_group_descriptor(
        &mut self,
        layout: RenderHandle<'g, BindGroupLayout>,
        dependencies: RenderDependencies<'g>,
        bind_group: impl FnOnce(NodeContext<'g>, &RenderDevice) -> Vec<BindGroupEntry<'g>>,
    ) -> RenderHandle<'g, BindGroup> {
        todo!();
    }

    #[inline]
    fn new_texture_direct(
        &mut self,
        descriptor: Option<TextureDescriptor<'static>>,
        texture: RefEq<'g, Texture>,
    ) -> RenderHandle<'g, Texture> {
        let id = self
            .graph
            .textures
            .new_direct(&mut self.graph.resources, descriptor, texture);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_texture_descriptor(
        &mut self,
        descriptor: TextureDescriptor<'static>,
    ) -> RenderHandle<'g, Texture> {
        let id = self
            .graph
            .textures
            .new_from_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_texture_view_direct(
        &mut self,
        source: Option<RenderHandle<'g, Texture>>,
        descriptor: Option<TextureViewDescriptor<'static>>,
        texture_view: RefEq<'g, TextureView>,
    ) -> RenderHandle<'g, TextureView> {
        todo!()
    }

    #[inline]
    fn new_texture_view_descriptor(
        &mut self,
        source: RenderHandle<'g, Texture>,
        descriptor: TextureViewDescriptor<'static>,
    ) -> RenderHandle<'g, TextureView> {
        todo!()
    }

    #[inline]
    fn new_sampler_direct(
        &mut self,
        descriptor: Option<RenderGraphSamplerDescriptor>,
        sampler: RefEq<'g, Sampler>,
    ) -> RenderHandle<'g, Sampler> {
        let id = self
            .graph
            .samplers
            .new_direct(&mut self.graph.resources, descriptor, sampler);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_sampler_descriptor(
        &mut self,
        descriptor: RenderGraphSamplerDescriptor,
    ) -> RenderHandle<'g, Sampler> {
        let id = self
            .graph
            .samplers
            .new_from_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_buffer_direct(
        &mut self,
        descriptor: Option<BufferDescriptor<'static>>,
        buffer: RefEq<'g, Buffer>,
    ) -> RenderHandle<'g, Buffer> {
        let id = self
            .graph
            .buffers
            .new_direct(&mut self.graph.resources, descriptor, buffer);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_buffer_descriptor(
        &mut self,
        descriptor: BufferDescriptor<'static>,
    ) -> RenderHandle<'g, Buffer> {
        let id = self
            .graph
            .buffers
            .new_from_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_render_pipeline_direct(
        &mut self,
        descriptor: Option<RenderPipelineDescriptor>,
        render_pipeline: RefEq<'g, RenderPipeline>,
    ) -> RenderHandle<'g, RenderPipeline> {
        todo!()
    }

    #[inline]
    fn new_render_pipeline_descriptor(
        &mut self,
        descriptor: RenderPipelineDescriptor,
    ) -> RenderHandle<'g, RenderPipeline> {
        todo!()
    }

    #[inline]
    fn new_compute_pipeline_direct(
        &mut self,
        descriptor: Option<ComputePipelineDescriptor>,
        compute_pipeline: RefEq<'g, ComputePipeline>,
    ) -> RenderHandle<'g, ComputePipeline> {
        todo!()
    }

    #[inline]
    fn new_compute_pipeline_descriptor(
        &mut self,
        descriptor: ComputePipelineDescriptor,
    ) -> RenderHandle<'g, ComputePipeline> {
        todo!()
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
