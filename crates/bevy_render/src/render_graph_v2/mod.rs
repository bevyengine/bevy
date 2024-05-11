pub mod resource;
mod setup;

use std::mem;

use crate::{
    render_resource::{
        BindGroup, BindGroupLayout, Buffer, ComputePipeline, ComputePipelineDescriptor,
        PipelineCache, RenderPipeline, RenderPipelineDescriptor, Sampler, Texture, TextureView,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{system::Resource, world::World};

use resource::{IntoRenderResource, RenderHandle, RenderResource, RenderResources};

use wgpu::{
    BindGroupEntry, BindGroupLayoutEntry, BufferDescriptor, CommandEncoder,
    CommandEncoderDescriptor, ComputePass, Label, RenderPass, TextureDescriptor,
    TextureViewDescriptor,
};

use self::resource::{
    bind_group::{RenderGraphBindGroupDescriptor, RenderGraphBindGroups},
    pipeline::{
        CachedRenderGraphPipelines, RenderGraphComputePipelineDescriptor, RenderGraphPipelines,
        RenderGraphRenderPipelineDescriptor,
    },
    ref_eq::RefEq,
    texture::{RenderGraphSamplerDescriptor, RenderGraphTextureView, RenderGraphTextureViews},
    CachedResources, DescribedRenderResource, RenderDependencies, RenderResourceGeneration,
    RenderResourceId, ResourceTracker, UsagesRenderResource,
};

#[derive(Resource, Default)]
struct RenderGraphCachedResources {
    bind_group_layouts: CachedResources<BindGroupLayout>,
    samplers: CachedResources<Sampler>,
    pipelines: CachedRenderGraphPipelines,
}

pub struct RenderGraph<'g> {
    resources: ResourceTracker<'g>,
    bind_group_layouts: RenderResources<'g, BindGroupLayout>,
    bind_groups: RenderGraphBindGroups<'g>,
    textures: RenderResources<'g, Texture>,
    texture_views: RenderGraphTextureViews<'g>,
    samplers: RenderResources<'g, Sampler>,
    buffers: RenderResources<'g, Buffer>,
    pipelines: RenderGraphPipelines<'g>,
    nodes: Vec<Node<'g>>,
    //TODO:: store node graph here
}

struct Node<'g> {
    label: Label<'g>,
    dependencies: RenderDependencies<'g>,
    runner: NodeRunner<'g>,
}

//For later auto-merging render passes
enum NodeRunner<'g> {
    Raw(Box<dyn FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut CommandEncoder) + 'g>),
    //Render(Box<dyn FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut RenderPass) + 'g>),
    Compute(Box<dyn FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut ComputePass) + 'g>),
}

impl<'g> RenderGraph<'g> {
    fn new() -> Self {
        Self {
            resources: ResourceTracker::default(),
            bind_group_layouts: RenderResources::new(|device, desc: &Vec<BindGroupLayoutEntry>| {
                device.create_bind_group_layout(None, desc)
            }),
            bind_groups: RenderGraphBindGroups::new(),
            textures: RenderResources::new(RenderDevice::create_texture),
            texture_views: RenderGraphTextureViews::new(),
            samplers: RenderResources::new(|device, RenderGraphSamplerDescriptor(desc)| {
                device.create_sampler(desc)
            }),
            buffers: RenderResources::new(RenderDevice::create_buffer),
            pipelines: RenderGraphPipelines::new(),
            nodes: Vec::new(),
        }
    }

    fn run(mut self, world: &World, render_device: &RenderDevice, render_queue: &RenderQueue) {
        let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("render_graph_command_encoder"),
        });
        for node in mem::take(&mut self.nodes) {
            //todo: profiling
            if self.resources.dependencies_ready(
                &self,
                world.resource::<PipelineCache>(),
                &node.dependencies,
            ) {
                let Node {
                    label,
                    dependencies,
                    runner,
                } = node;
                let context = NodeContext {
                    graph: &self,
                    world,
                    dependencies,
                    // entity: entity_ref,
                };
                match runner {
                    NodeRunner::Raw(f) => (f)(context, render_device, render_queue, &mut encoder),
                    // NodeRunner::Render(f) => {
                    //     let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { label:  gg, color_attachments: (), depth_stencil_attachment: (), timestamp_writes: (), occlusion_query_set: () })
                    // },
                    NodeRunner::Compute(f) => {
                        let mut compute_pass =
                            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label,
                                timestamp_writes: None,
                            });
                        (f)(context, render_device, render_queue, &mut compute_pass)
                    }
                }
            }
        }
        render_queue.submit([encoder.finish()]);
    }

    fn generation(&self, id: RenderResourceId) -> RenderResourceGeneration {
        self.resources.generation(id)
    }

    fn create_queued_resources(
        &mut self,
        resource_cache: &mut RenderGraphCachedResources,
        pipeline_cache: &mut PipelineCache,
        render_device: &RenderDevice,
        world: &World,
        // view_entity: EntityRef<'g>,
    ) {
        self.bind_group_layouts
            .create_queued_resources_cached(&mut resource_cache.bind_group_layouts, render_device);

        let mut pipelines = mem::take(&mut self.pipelines);
        pipelines.create_queued_pipelines(
            self,
            &mut resource_cache.pipelines,
            pipeline_cache,
            world,
        );
        self.pipelines = pipelines;

        self.textures.create_queued_resources(render_device);

        let mut texture_views = std::mem::take(&mut self.texture_views);
        texture_views.create_queued_resources(self, world /*, view_entity*/);
        self.texture_views = texture_views;

        self.samplers
            .create_queued_resources_cached(&mut resource_cache.samplers, render_device);
        self.buffers.create_queued_resources(render_device);

        let mut bind_groups = std::mem::take(&mut self.bind_groups);
        bind_groups.create_queued_bind_groups(self, world, render_device /*, view_entity*/);
        self.bind_groups = bind_groups;
    }
}

pub struct RenderGraphBuilder<'g> {
    graph: RenderGraph<'g>,
    resource_cache: &'g mut RenderGraphCachedResources,
    pipeline_cache: &'g mut PipelineCache,
    world: &'g World,
    // view_entity: EntityRef<'g>,
    render_device: &'g RenderDevice,
}

impl<'g> RenderGraphBuilder<'g> {
    pub fn new_resource<R: IntoRenderResource<'g>>(
        &mut self,
        resource: R,
    ) -> RenderHandle<'g, R::Resource> {
        R::into_render_resource(resource, self)
    }

    pub fn into_resource<R: RenderResource>(&mut self, resource: R) -> RenderHandle<'g, R> {
        self.new_resource(RefEq::Owned(resource))
    }

    pub fn as_resource<R: RenderResource>(&mut self, resource: &'g R) -> RenderHandle<'g, R> {
        self.new_resource(RefEq::Borrowed(resource))
    }

    pub fn get_descriptor_of<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> Option<&R::Descriptor> {
        R::get_descriptor(self, resource)
    }

    pub fn descriptor_of<R: DescribedRenderResource>(
        &self,
        resource: RenderHandle<'g, R>,
    ) -> &R::Descriptor {
        self.get_descriptor_of(resource)
            .unwrap_or_else(|| panic!("Descriptor not found for resource: {:?}", resource))
    }

    pub fn get_layout(
        &self,
        bind_group: RenderHandle<'g, BindGroup>,
    ) -> Option<RenderHandle<'g, BindGroupLayout>> {
        self.graph.bind_groups.get_layout(bind_group.id())
    }

    pub fn layout(
        &self,
        bind_group: RenderHandle<'g, BindGroup>,
    ) -> RenderHandle<'g, BindGroupLayout> {
        self.get_layout(bind_group).unwrap_or_else(|| {
            panic!(
                "No BindGroupLayout handle associated with BindGroup: {:?}",
                bind_group
            )
        })
    }

    pub fn is_fresh<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> bool {
        self.graph.generation(resource.id()) == 0
    }

    pub fn add_usages<R: UsagesRenderResource>(
        &mut self,
        resource: RenderHandle<'g, R>,
        usages: R::Usages,
    ) -> &mut Self {
        let desc = R::get_descriptor_mut(self, resource);
        if let Some(desc) = desc {
            R::add_usages(desc, usages);
        } else {
            let has_usages = R::get_descriptor(self, resource)
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
        label: Label<'g>,
        dependencies: RenderDependencies<'g>,
        node: impl FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut CommandEncoder) + 'g,
    ) -> &mut Self {
        //get + save dependency generations here, since they're not stored in RenderDependencies.
        //This is to make creating a RenderDependencies (and cloning!) a pure operation.
        self.graph.resources.write_dependencies(&dependencies);
        self.graph.nodes.push(Node {
            label,
            dependencies,
            runner: NodeRunner::Raw(Box::new(node)),
        });

        self
    }

    pub fn add_compute_node(
        &mut self,
        label: Label<'g>,
        dependencies: RenderDependencies<'g>,
        node: impl FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut ComputePass) + 'g,
    ) -> &mut Self {
        //get + save dependency generations here, since they're not stored in RenderDependencies.
        //This is to make creating a RenderDependencies (and cloning!) a pure operation.
        self.graph.resources.write_dependencies(&dependencies);
        self.graph.nodes.push(Node {
            label,
            dependencies,
            runner: NodeRunner::Compute(Box::new(node)),
        });

        self
    }

    pub fn features(&self) -> wgpu::Features {
        self.render_device.features()
    }

    pub fn limits(&self) -> wgpu::Limits {
        self.render_device.limits()
    }

    fn create_queued_resources(&mut self) {
        self.graph.create_queued_resources(
            self.resource_cache,
            self.pipeline_cache,
            self.render_device,
            self.world,
        );
    }

    fn run(self, render_queue: &RenderQueue) {
        self.graph.run(self.world, self.render_device, render_queue);
    }
}

impl<'g> RenderGraphBuilder<'g> {
    pub fn world_resource<R: Resource>(&self) -> &'g R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&self) -> Option<&'g R> {
        self.world.get_resource()
    }

    // pub fn view_id(&self) -> Entity {
    //     self.view_entity.id()
    // }
    //
    // pub fn view_contains<C: Component>(&self) -> bool {
    //     self.view_entity.contains::<C>()
    // }
    //
    // pub fn view_get<C: Component>(&self) -> Option<&'g C> {
    //     self.view_entity.get()
    // }
    //
    // pub fn view_get_ref<C: Component>(&self) -> Option<Ref<'g, C>> {
    //     self.view_entity.get_ref()
    // }
    //
    // pub fn view_entity(&self) -> EntityRef<'g> {
    //     self.view_entity
    // }

    pub fn world(&self) -> &'g World {
        self.world
    }
}

impl<'g> RenderGraphBuilder<'g> {
    #[inline]
    fn new_bind_group_layout_direct(
        &mut self,
        descriptor: Option<Vec<BindGroupLayoutEntry>>,
        bind_group_layout: RefEq<'g, BindGroupLayout>,
    ) -> RenderHandle<'g, BindGroupLayout> {
        let id = self.graph.bind_group_layouts.new_direct(
            &mut self.graph.resources,
            descriptor,
            bind_group_layout,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_bind_group_layout_descriptor(
        &mut self,
        descriptor: Vec<BindGroupLayoutEntry>,
    ) -> RenderHandle<'g, BindGroupLayout> {
        let id = self
            .graph
            .bind_group_layouts
            .new_from_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_bind_group_layout_descriptor(
        &self,
        bind_group_layout: RenderHandle<'g, BindGroupLayout>,
    ) -> Option<&Vec<BindGroupLayoutEntry>> {
        self.graph
            .bind_group_layouts
            .get_descriptor(bind_group_layout.id())
    }

    #[inline]
    fn new_bind_group_direct(
        &mut self,
        dependencies: RenderDependencies<'g>,
        layout: Option<RenderHandle<'g, BindGroupLayout>>,
        bind_group: RefEq<'g, BindGroup>,
    ) -> RenderHandle<'g, BindGroup> {
        let id = self.graph.bind_groups.new_direct(
            &mut self.graph.resources,
            dependencies,
            layout,
            bind_group,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_bind_group_descriptor(
        &mut self,
        descriptor: RenderGraphBindGroupDescriptor<'g>,
    ) -> RenderHandle<'g, BindGroup> {
        let id = self
            .graph
            .bind_groups
            .new_from_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
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
    fn get_texture_descriptor(
        &self,
        texture: RenderHandle<'g, Texture>,
    ) -> Option<&TextureDescriptor<'static>> {
        self.graph.textures.get_descriptor(texture.id())
    }

    #[inline]
    fn get_texture_descriptor_mut(
        &mut self,
        texture: RenderHandle<'g, Texture>,
    ) -> Option<&mut TextureDescriptor<'static>> {
        self.graph.textures.get_descriptor_mut(texture.id())
    }

    #[inline]
    fn new_texture_view_direct(
        &mut self,
        descriptor: Option<TextureViewDescriptor<'static>>,
        texture_view: RefEq<'g, TextureView>,
    ) -> RenderHandle<'g, TextureView> {
        let id = self.graph.texture_views.new_direct(
            &mut self.graph.resources,
            descriptor,
            texture_view,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_texture_view_descriptor(
        &mut self,
        texture_view: RenderGraphTextureView<'g>,
    ) -> RenderHandle<'g, TextureView> {
        let id = self
            .graph
            .texture_views
            .new_from_descriptor(&mut self.graph.resources, texture_view);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_texture_view_descriptor(
        &self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> Option<&TextureViewDescriptor<'static>> {
        self.graph.texture_views.get_descriptor(texture_view.id())
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
    fn get_sampler_descriptor(
        &self,
        sampler: RenderHandle<'g, Sampler>,
    ) -> Option<&RenderGraphSamplerDescriptor> {
        self.graph.samplers.get_descriptor(sampler.id())
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
    fn get_buffer_descriptor(
        &self,
        buffer: RenderHandle<'g, Buffer>,
    ) -> Option<&BufferDescriptor<'static>> {
        self.graph.buffers.get_descriptor(buffer.id())
    }

    #[inline]
    fn get_buffer_descriptor_mut(
        &mut self,
        buffer: RenderHandle<'g, Buffer>,
    ) -> Option<&mut BufferDescriptor<'static>> {
        self.graph.buffers.get_descriptor_mut(buffer.id())
    }

    #[inline]
    fn new_render_pipeline_direct(
        &mut self,
        descriptor: Option<RenderPipelineDescriptor>,
        render_pipeline: RefEq<'g, RenderPipeline>,
    ) -> RenderHandle<'g, RenderPipeline> {
        let id = self.graph.pipelines.new_render_pipeline_direct(
            &mut self.graph.resources,
            descriptor,
            render_pipeline,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_render_pipeline_descriptor(
        &mut self,
        descriptor: RenderGraphRenderPipelineDescriptor<'g>,
    ) -> RenderHandle<'g, RenderPipeline> {
        let id = self
            .graph
            .pipelines
            .new_render_pipeline_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_render_pipeline_descriptor(
        &self,
        render_pipeline: RenderHandle<'g, RenderPipeline>,
    ) -> Option<&RenderPipelineDescriptor> {
        self.graph.pipelines.get_render_pipeline_descriptor(
            self.world_resource::<PipelineCache>(),
            render_pipeline.id(),
        )
    }

    #[inline]
    fn new_compute_pipeline_direct(
        &mut self,
        descriptor: Option<ComputePipelineDescriptor>,
        compute_pipeline: RefEq<'g, ComputePipeline>,
    ) -> RenderHandle<'g, ComputePipeline> {
        let id = self.graph.pipelines.new_compute_pipeline_direct(
            &mut self.graph.resources,
            descriptor,
            compute_pipeline,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_compute_pipeline_descriptor(
        &mut self,
        descriptor: RenderGraphComputePipelineDescriptor<'g>,
    ) -> RenderHandle<'g, ComputePipeline> {
        let id = self
            .graph
            .pipelines
            .new_compute_pipeline_descriptor(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_compute_pipeline_descriptor(
        &self,
        compute_pipeline: RenderHandle<'g, ComputePipeline>,
    ) -> Option<&ComputePipelineDescriptor> {
        self.graph.pipelines.get_compute_pipeline_descriptor(
            self.world_resource::<PipelineCache>(),
            compute_pipeline.id(),
        )
    }
}

#[derive(Clone)]
pub struct NodeContext<'g> {
    graph: &'g RenderGraph<'g>,
    world: &'g World,
    dependencies: RenderDependencies<'g>,
    // entity: EntityRef<'g>,
}

impl<'g> NodeContext<'g> {
    pub fn get<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> &R {
        if !self.dependencies.includes(resource) {
            panic!(
                "Illegal resource access: {:?}. Have you added it to the node's dependencies?",
                resource
            );
        }

        R::get_from_store(self, resource)
            .unwrap_or_else(|| panic!("Unable to locate render graph resource: {:?}", resource))
    }

    fn get_texture(&self, texture: RenderHandle<'g, Texture>) -> Option<&Texture> {
        self.graph.textures.get(texture.id())
    }

    fn get_texture_view(
        &self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> Option<&TextureView> {
        self.graph.texture_views.get(texture_view.id())
    }

    fn get_sampler(&self, sampler: RenderHandle<'g, Sampler>) -> Option<&Sampler> {
        self.graph.samplers.get(sampler.id())
    }

    fn get_buffer(&self, buffer: RenderHandle<'g, Buffer>) -> Option<&Buffer> {
        self.graph.buffers.get(buffer.id())
    }

    fn get_render_pipeline(
        &self,
        render_pipeline: RenderHandle<'g, RenderPipeline>,
    ) -> Option<&RenderPipeline> {
        let pipeline_cache = self.world.resource::<PipelineCache>();
        self.graph
            .pipelines
            .get_render_pipeline(pipeline_cache, render_pipeline.id())
    }

    fn get_compute_pipeline(
        &self,
        compute_pipeline: RenderHandle<'g, ComputePipeline>,
    ) -> Option<&ComputePipeline> {
        let pipeline_cache = self.world.resource::<PipelineCache>();
        self.graph
            .pipelines
            .get_compute_pipeline(pipeline_cache, compute_pipeline.id())
    }

    fn get_bind_group_layout(
        &self,
        bind_group_layout: RenderHandle<'g, BindGroupLayout>,
    ) -> Option<&BindGroupLayout> {
        self.graph.bind_group_layouts.get(bind_group_layout.id())
    }

    fn get_bind_group(&self, bind_group: RenderHandle<'g, BindGroup>) -> Option<&BindGroup> {
        self.graph.bind_groups.get(bind_group.id())
    }
}

impl<'g> NodeContext<'g> {
    pub fn world_resource<R: Resource>(&self) -> &R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&self) -> Option<&'g R> {
        self.world.get_resource()
    }

    // pub fn view_id(&self) -> Entity {
    //     self.entity.id()
    // }
    //
    // pub fn view_contains<C: Component>(&self) -> bool {
    //     self.entity.contains::<C>()
    // }
    //
    // pub fn view_get<C: Component>(&self) -> Option<&'g C> {
    //     self.entity.get()
    // }
    //
    // pub fn view_get_ref<C: Component>(&self) -> Option<Ref<'g, C>> {
    //     self.entity.get_ref()
    // }
    //
    // pub fn view_entity(&'g self) -> EntityRef<'g> {
    //     self.entity
    // }

    pub fn world(&'g self) -> &'g World {
        self.world
    }
}
