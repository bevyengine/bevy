pub mod debug;
pub mod resource;
mod setup;

pub use setup::RenderGraphPlugin;

use bevy_utils::HashMap;
pub use setup::RenderGraphSetup;

use std::{borrow::Cow, mem};

use bevy_ecs::{system::Resource, world::World};
use bevy_render::{
    render_resource::{
        BindGroup, BindGroupLayout, Buffer, CommandEncoder, CommandEncoderDescriptor, ComputePass,
        ComputePassDescriptor, ComputePipeline, PipelineCache, RenderPipeline, Sampler, Texture,
        TextureDescriptor, TextureView,
    },
    renderer::{RenderDevice, RenderQueue},
    settings::{WgpuFeatures, WgpuLimits},
};

use resource::{IntoRenderResource, RenderHandle, RenderResource, RenderResources};

use crate::{core::debug::RenderGraphDebug, deps};

use self::{
    debug::{RenderGraphDebugContext, RenderGraphDebugWrapper},
    resource::{
        make_bind_group, CachedRenderGraphPipelines, CachedResources, RenderDependencies,
        RenderGraphBindGroupLayoutMeta, RenderGraphBindGroupMeta, RenderGraphBufferMeta,
        RenderGraphComputePipelineDescriptor, RenderGraphPipelines,
        RenderGraphRenderPipelineDescriptor, RenderGraphSamplerDescriptor,
        RenderGraphTextureViewDescriptor, RenderResourceGeneration, RenderResourceId,
        ResourceTracker, UsagesRenderResource,
    },
};

pub type Label<'a> = Option<Cow<'a, str>>;

#[derive(Resource, Default)]
struct RenderGraphCachedResources {
    bind_group_layouts: CachedResources<BindGroupLayout>,
    samplers: CachedResources<Sampler>,
    pipelines: CachedRenderGraphPipelines,
}

#[derive(Default)]
pub struct RenderGraph<'g> {
    resources: ResourceTracker<'g>,
    bind_group_layouts: RenderResources<'g, BindGroupLayout>,
    bind_groups: RenderResources<'g, BindGroup>,
    textures: RenderResources<'g, Texture>,
    texture_views: RenderResources<'g, TextureView>,
    samplers: RenderResources<'g, Sampler>,
    buffers: RenderResources<'g, Buffer>,
    pipelines: RenderGraphPipelines<'g>,
    nodes: Vec<Node<'g>>,
}

struct Node<'g> {
    label: Label<'g>,
    dependencies: RenderDependencies<'g>,
    runner: NodeRunner<'g>,
}

#[allow(clippy::type_complexity)]
enum NodeRunner<'g> {
    Raw(Box<dyn FnOnce(NodeContext<'_, 'g>, &mut CommandEncoder, &RenderDevice) + Send + 'g>),
    //todo: possibility of auto-merging render passes?
    //Render(Box<dyn FnOnce(NodeContext, &RenderDevice, &RenderQueue, &mut RenderPass) + 'g>),
    Compute(Box<dyn for<'n> FnOnce(&'n NodeContext<'n, 'g>, &mut ComputePass<'n>) + Send + 'g>),
}

impl<'g> RenderGraph<'g> {
    fn new() -> Self {
        Default::default()
    }

    fn run(
        mut self,
        world: &'g World,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        pipeline_cache: &PipelineCache,
    ) {
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
                    pipeline_cache: Some(pipeline_cache),
                };

                match runner {
                    NodeRunner::Raw(f) => (f)(context, &mut encoder, render_device),
                    // NodeRunner::Render(f) => {
                    //     let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { label: gg, color_attachments: (), depth_stencil_attachment: (), timestamp_writes: (), occlusion_query_set: () })
                    // },
                    NodeRunner::Compute(f) => {
                        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                            label: label.as_deref(),
                            timestamp_writes: None,
                        });
                        (f)(&context, &mut compute_pass);
                    }
                }
            }
        }
        render_queue.submit([encoder.finish()]);
    }

    fn label(&self, id: RenderResourceId) -> &Label<'g> {
        self.resources.label(id)
    }

    fn as_debug_ctx(&self) -> RenderGraphDebugContext<'_, 'g> {
        RenderGraphDebugContext(self)
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
        let mut bind_group_layouts = mem::take(&mut self.bind_group_layouts);
        bind_group_layouts.create_queued_resources_cached(
            &mut resource_cache.bind_group_layouts,
            world,
            render_device,
            self,
            |_, render_device, _, meta| {
                render_device.create_bind_group_layout(
                    meta.descriptor.label.as_deref(),
                    &meta.descriptor.entries,
                )
            },
        );
        self.bind_group_layouts = bind_group_layouts;

        //MUST be after bind group layouts
        let mut pipelines = mem::take(&mut self.pipelines);
        pipelines.create_queued_pipelines(
            self,
            &mut resource_cache.pipelines,
            pipeline_cache,
            world,
        );
        self.pipelines = pipelines;

        let mut textures = mem::take(&mut self.textures);
        textures.create_queued_resources(
            world,
            render_device,
            self,
            |_, render_device, _, descriptor| render_device.create_texture(descriptor),
        );
        self.textures = textures;

        let mut samplers = mem::take(&mut self.samplers);
        samplers.create_queued_resources_cached(
            &mut resource_cache.samplers,
            world,
            render_device,
            self,
            |_, render_device, _, descriptor| render_device.create_sampler(&descriptor.0),
        );
        self.samplers = samplers;

        //MUST be after textures
        let mut texture_views = mem::take(&mut self.texture_views);
        let mut texture_view_cache = HashMap::new();
        texture_views.create_queued_resources(
            world,
            render_device,
            self,
            |world, _, render_graph, meta| {
                let mut deps = RenderDependencies::new();
                deps.write(meta.texture);
                let context = NodeContext {
                    graph: render_graph,
                    world,
                    dependencies: deps,
                    pipeline_cache: None,
                };
                texture_view_cache
                    .entry(meta.clone())
                    .or_insert_with(|| context.get(meta.texture).create_view(&meta.descriptor))
                    .clone() //TODO: hopefully unnecessary clone
            },
        );
        self.texture_views = texture_views;

        let mut buffers = mem::take(&mut self.buffers);
        buffers.create_queued_resources(world, render_device, self, |_, render_device, _, meta| {
            render_device.create_buffer(&meta.descriptor)
        });
        self.buffers = buffers;

        //MUST be last
        let mut bind_groups = mem::take(&mut self.bind_groups);
        let mut bind_group_cache = HashMap::new();
        bind_groups.create_queued_resources(
            world,
            render_device,
            self,
            |world, render_device, graph, meta| {
                let context = NodeContext {
                    graph,
                    world,
                    dependencies: meta.dependencies(),
                    pipeline_cache: None,
                };
                bind_group_cache
                    .entry(meta.descriptor.clone()) //TODO: hopefully unnecessary clone
                    .or_insert_with(|| make_bind_group(&context, render_device, &meta.descriptor))
                    .clone()
            },
        );
        self.bind_groups = bind_groups;
    }

    fn borrow_cached_resources(&mut self, resource_cache: &'g RenderGraphCachedResources) {
        self.bind_group_layouts
            .borrow_cached_resources(&resource_cache.bind_group_layouts);
        self.samplers
            .borrow_cached_resources(&resource_cache.samplers);
    }
}

pub struct RenderGraphBuilder<'b, 'g: 'b> {
    graph: &'b mut RenderGraph<'g>,
    // resource_cache: &'b mut RenderGraphCachedResources,
    // pipeline_cache: &'b mut PipelineCache,
    world: &'g World,
    // view_entity: EntityRef<'g>,
    render_device: &'b RenderDevice,
}

impl<'b, 'g: 'b> RenderGraphBuilder<'b, 'g> {
    #[inline]
    pub fn new_resource<R: IntoRenderResource<'g>>(
        &mut self,
        resource: R,
    ) -> RenderHandle<'g, R::Resource> {
        R::into_render_resource(resource, self)
    }

    #[inline]
    pub fn into_resource<R: RenderResource>(
        &mut self,
        meta: R::Meta<'g>,
        resource: R,
    ) -> RenderHandle<'g, R> {
        R::import_resource(self, meta, Cow::Owned(resource))
    }

    #[inline]
    pub fn as_resource<R: RenderResource>(
        &mut self,
        meta: R::Meta<'g>,
        resource: &'g R,
    ) -> RenderHandle<'g, R> {
        R::import_resource(self, meta, Cow::Borrowed(resource))
    }

    #[inline]
    pub fn meta<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> &R::Meta<'g> {
        R::get_meta(self, resource).unwrap_or_else(|| {
            panic!(
                "No descriptor found for resource: {:?}",
                self.debug(&resource)
            )
        })
    }

    #[inline]
    pub fn is_fresh<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> bool {
        self.graph.generation(resource.id()) == 0
    }

    pub fn add_usages<R: UsagesRenderResource>(
        &mut self,
        resource: RenderHandle<'g, R>,
        usages: R::Usages,
    ) -> &mut Self {
        let desc = R::get_meta_mut(self, resource);
        if let Some(desc) = desc {
            R::add_usages(desc, usages);
        } else if !R::has_usages(self.meta(resource), &usages) {
            panic!(
                "Descriptor for resource {:?} does not contain necessary usages: {:?}",
                self.debug(&resource),
                usages
            )
        }
        self
    }

    pub fn add_node(
        &mut self,
        label: Label<'g>,
        dependencies: RenderDependencies<'g>,
        node: impl FnOnce(NodeContext<'_, 'g>, &mut CommandEncoder, &RenderDevice) + Send + 'g,
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
        node: impl for<'n> FnOnce(&'n NodeContext<'n, 'g>, &mut ComputePass<'n>) + Send + 'g,
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

    #[inline]
    pub fn features(&self) -> WgpuFeatures {
        self.render_device.features()
    }

    #[inline]
    pub fn limits(&self) -> WgpuLimits {
        self.render_device.limits()
    }

    #[inline]
    pub fn debug<'a, T: RenderGraphDebug<'g>>(
        &'a self,
        value: &'a T,
    ) -> RenderGraphDebugWrapper<'a, 'g, T> {
        self.graph.as_debug_ctx().debug(value)
    }
}

impl<'g> RenderGraphBuilder<'_, 'g> {
    #[inline]
    pub fn world_resource<R: Resource>(&self) -> &'g R {
        self.world.resource()
    }

    #[inline]
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

    #[inline]
    pub fn world(&self) -> &'g World {
        self.world
    }
}

impl<'b, 'g: 'b> RenderGraphBuilder<'b, 'g> {
    #[inline]
    fn import_bind_group_layout(
        &mut self,
        descriptor: RenderGraphBindGroupLayoutMeta,
        bind_group_layout: Cow<'g, BindGroupLayout>,
    ) -> RenderHandle<'g, BindGroupLayout> {
        let id = self.graph.bind_group_layouts.import_resource(
            &mut self.graph.resources,
            None,
            descriptor,
            bind_group_layout,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_bind_group_layout(
        &mut self,
        mut descriptor: RenderGraphBindGroupLayoutMeta,
    ) -> RenderHandle<'g, BindGroupLayout> {
        descriptor
            .descriptor
            .entries
            .sort_by_key(|entry| entry.binding);
        let id =
            self.graph
                .bind_group_layouts
                .new_resource(&mut self.graph.resources, None, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_bind_group_layout_meta(
        &self,
        bind_group_layout: RenderHandle<'g, BindGroupLayout>,
    ) -> Option<&RenderGraphBindGroupLayoutMeta> {
        self.graph
            .bind_group_layouts
            .get_meta(bind_group_layout.id())
    }

    #[inline]
    fn import_bind_group(
        &mut self,
        descriptor: RenderGraphBindGroupMeta<'g>,
        bind_group: Cow<'g, BindGroup>,
    ) -> RenderHandle<'g, BindGroup> {
        let id = self.graph.bind_groups.import_resource(
            &mut self.graph.resources,
            Some(descriptor.dependencies()),
            descriptor,
            bind_group,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_bind_group(
        &mut self,
        mut meta: RenderGraphBindGroupMeta<'g>,
    ) -> RenderHandle<'g, BindGroup> {
        meta.descriptor.entries.sort_by_key(|entry| entry.binding);
        let id = self.graph.bind_groups.new_resource(
            &mut self.graph.resources,
            Some(meta.dependencies()),
            meta,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn get_bind_group_meta(
        &self,
        bind_group: RenderHandle<'g, BindGroup>,
    ) -> Option<&RenderGraphBindGroupMeta<'g>> {
        self.graph.bind_groups.get_meta(bind_group.id())
    }

    #[inline]
    fn import_texture(
        &mut self,
        descriptor: TextureDescriptor<'static>,
        texture: Cow<'g, Texture>,
    ) -> RenderHandle<'g, Texture> {
        let id = self.graph.textures.import_resource(
            &mut self.graph.resources,
            None,
            descriptor,
            texture,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_texture(&mut self, descriptor: TextureDescriptor<'static>) -> RenderHandle<'g, Texture> {
        let id = self
            .graph
            .textures
            .new_resource(&mut self.graph.resources, None, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_texture_meta(
        &self,
        texture: RenderHandle<'g, Texture>,
    ) -> Option<&TextureDescriptor<'static>> {
        self.graph.textures.get_meta(texture.id())
    }

    #[inline]
    fn get_texture_meta_mut(
        &mut self,
        texture: RenderHandle<'g, Texture>,
    ) -> Option<&mut TextureDescriptor<'static>> {
        self.graph.textures.get_meta_mut(texture.id())
    }

    #[inline]
    fn import_texture_view(
        &mut self,
        mut descriptor: RenderGraphTextureViewDescriptor<'g>,
        texture_view: Cow<'g, TextureView>,
    ) -> RenderHandle<'g, TextureView> {
        let id = self.graph.texture_views.import_resource(
            &mut self.graph.resources,
            Some(deps![&mut descriptor.texture]),
            descriptor,
            texture_view,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_texture_view(
        &mut self,
        mut descriptor: RenderGraphTextureViewDescriptor<'g>,
    ) -> RenderHandle<'g, TextureView> {
        let id = self.graph.texture_views.new_resource(
            &mut self.graph.resources,
            Some(deps![&mut descriptor.texture]),
            descriptor,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn get_texture_view_meta(
        &self,
        texture_view: RenderHandle<'g, TextureView>,
    ) -> Option<&RenderGraphTextureViewDescriptor<'g>> {
        self.graph.texture_views.get_meta(texture_view.id())
    }

    #[inline]
    fn import_sampler(
        &mut self,
        descriptor: RenderGraphSamplerDescriptor,
        sampler: Cow<'g, Sampler>,
    ) -> RenderHandle<'g, Sampler> {
        let id = self.graph.samplers.import_resource(
            &mut self.graph.resources,
            None,
            descriptor,
            sampler,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_sampler(
        &mut self,
        descriptor: RenderGraphSamplerDescriptor,
    ) -> RenderHandle<'g, Sampler> {
        let id = self
            .graph
            .samplers
            .new_resource(&mut self.graph.resources, None, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_sampler_meta(
        &self,
        sampler: RenderHandle<'g, Sampler>,
    ) -> Option<&RenderGraphSamplerDescriptor> {
        self.graph.samplers.get_meta(sampler.id())
    }

    #[inline]
    fn import_buffer(
        &mut self,
        meta: RenderGraphBufferMeta,
        buffer: Cow<'g, Buffer>,
    ) -> RenderHandle<'g, Buffer> {
        let id = self
            .graph
            .buffers
            .import_resource(&mut self.graph.resources, None, meta, buffer);
        RenderHandle::new(id)
    }

    #[inline]
    fn new_buffer(&mut self, meta: RenderGraphBufferMeta) -> RenderHandle<'g, Buffer> {
        let id = self
            .graph
            .buffers
            .new_resource(&mut self.graph.resources, None, meta);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_buffer_meta(&self, buffer: RenderHandle<'g, Buffer>) -> Option<&RenderGraphBufferMeta> {
        self.graph.buffers.get_meta(buffer.id())
    }

    #[inline]
    fn get_buffer_meta_mut(
        &mut self,
        buffer: RenderHandle<'g, Buffer>,
    ) -> Option<&mut RenderGraphBufferMeta> {
        self.graph.buffers.get_meta_mut(buffer.id())
    }

    #[inline]
    fn import_render_pipeline(
        &mut self,
        descriptor: RenderGraphRenderPipelineDescriptor<'g>,
        render_pipeline: Cow<'g, RenderPipeline>,
    ) -> RenderHandle<'g, RenderPipeline> {
        let id = self.graph.pipelines.import_render_pipeline(
            &mut self.graph.resources,
            descriptor,
            render_pipeline,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_render_pipeline(
        &mut self,
        descriptor: RenderGraphRenderPipelineDescriptor<'g>,
    ) -> RenderHandle<'g, RenderPipeline> {
        let id = self
            .graph
            .pipelines
            .new_render_pipeline(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_render_pipeline_meta(
        &self,
        render_pipeline: RenderHandle<'g, RenderPipeline>,
    ) -> Option<&RenderGraphRenderPipelineDescriptor<'g>> {
        self.graph
            .pipelines
            .get_render_pipeline_meta(render_pipeline.id())
    }

    #[inline]
    fn import_compute_pipeline(
        &mut self,
        descriptor: RenderGraphComputePipelineDescriptor<'g>,
        compute_pipeline: Cow<'g, ComputePipeline>,
    ) -> RenderHandle<'g, ComputePipeline> {
        let id = self.graph.pipelines.import_compute_pipeline(
            &mut self.graph.resources,
            descriptor,
            compute_pipeline,
        );
        RenderHandle::new(id)
    }

    #[inline]
    fn new_compute_pipeline(
        &mut self,
        descriptor: RenderGraphComputePipelineDescriptor<'g>,
    ) -> RenderHandle<'g, ComputePipeline> {
        let id = self
            .graph
            .pipelines
            .new_compute_pipeline(&mut self.graph.resources, descriptor);
        RenderHandle::new(id)
    }

    #[inline]
    fn get_compute_pipeline_meta(
        &self,
        compute_pipeline: RenderHandle<'g, ComputePipeline>,
    ) -> Option<&RenderGraphComputePipelineDescriptor<'g>> {
        self.graph
            .pipelines
            .get_compute_pipeline_meta(compute_pipeline.id())
    }
}

#[derive(Clone)]
pub struct NodeContext<'n, 'g: 'n> {
    graph: &'n RenderGraph<'g>,
    world: &'n World,
    dependencies: RenderDependencies<'g>,
    pipeline_cache: Option<&'n PipelineCache>, //Jank
}

impl<'n, 'g: 'n> NodeContext<'n, 'g> {
    pub fn get<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> &R {
        if !self.dependencies.includes(resource) {
            panic!(
                "Illegal resource access: {:?}. Have you added it to the node's dependencies?",
                self.debug(&resource)
            );
        }

        R::get(self, resource).unwrap_or_else(|| {
            panic!(
                "Unable to locate render graph resource: {:?}",
                self.debug(&resource)
            )
        })
    }

    pub fn debug<'a, T: RenderGraphDebug<'g>>(
        &'a self,
        value: &'a T,
    ) -> RenderGraphDebugWrapper<'a, 'g, T> {
        self.graph.as_debug_ctx().debug(value)
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
        self.graph.pipelines.get_render_pipeline(
            self.pipeline_cache
                .expect("No PipelineCache present in NodeContext"),
            render_pipeline.id(),
        )
    }

    fn get_compute_pipeline(
        &self,
        compute_pipeline: RenderHandle<'g, ComputePipeline>,
    ) -> Option<&ComputePipeline> {
        self.graph.pipelines.get_compute_pipeline(
            self.pipeline_cache
                .expect("No PipelineCache present in NodeContext"),
            compute_pipeline.id(),
        )
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

impl<'n, 'g: 'n> NodeContext<'n, 'g> {
    pub fn world_resource<R: Resource>(&self) -> &'n R {
        self.world.resource()
    }

    pub fn get_world_resource<R: Resource>(&self) -> Option<&'n R> {
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

    pub fn world(&self) -> &World {
        self.world
    }
}
