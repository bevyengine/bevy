use super::{
    wgpu_type_converter::{OwnedWgpuVertexBufferDescriptor, WgpuInto},
    WgpuRenderPass, WgpuResources,
};
use crate::renderer_2::{
    WgpuRenderContext, WgpuRenderResourceContext, WgpuTransactionalRenderResourceContext, render_resource_sets_system,
};
use bevy_app::{EventReader, Events};
use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    pass::{
        PassDescriptor, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor,
    },
    pipeline::{update_shader_assignments, PipelineCompiler, PipelineDescriptor},
    render_graph::RenderGraph,
    render_resource::{
        resource_name, AssetResources, BufferInfo, RenderResource, RenderResourceAssignments,
        ResourceInfo,
    },
    renderer::Renderer,
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_window::{WindowCreated, WindowResized, Windows};
use legion::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

pub struct WgpuRenderer {
    pub global_context: WgpuRenderContext<WgpuRenderResourceContext>,
    pub queue: wgpu::Queue,
    pub encoder: Option<wgpu::CommandEncoder>,
    pub render_pipelines: HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>,
    pub window_resized_event_reader: EventReader<WindowResized>,
    pub window_created_event_reader: EventReader<WindowCreated>,
    pub intialized: bool,
}

impl WgpuRenderer {
    pub async fn new(
        window_resized_event_reader: EventReader<WindowResized>,
        window_created_event_reader: EventReader<WindowCreated>,
    ) -> Self {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: None,
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits::default(),
            })
            .await;
        let device = Arc::new(device);
        WgpuRenderer {
            global_context: WgpuRenderContext::new(
                device.clone(),
                WgpuRenderResourceContext::new(device),
            ),
            queue,
            encoder: None,
            window_resized_event_reader,
            window_created_event_reader,
            intialized: false,
            render_pipelines: HashMap::new(),
        }
    }

    pub fn create_render_pass<'a, 'b>(
        wgpu_resources: &'a WgpuResources,
        pass_descriptor: &PassDescriptor,
        global_render_resource_assignments: &'b RenderResourceAssignments,
        encoder: &'a mut wgpu::CommandEncoder,
        primary_swap_chain: &Option<String>,
        swap_chain_outputs: &'a HashMap<String, wgpu::SwapChainOutput>,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &pass_descriptor
                .color_attachments
                .iter()
                .map(|c| {
                    Self::create_wgpu_color_attachment_descriptor(
                        wgpu_resources,
                        global_render_resource_assignments,
                        c,
                        primary_swap_chain,
                        swap_chain_outputs,
                    )
                })
                .collect::<Vec<wgpu::RenderPassColorAttachmentDescriptor>>(),
            depth_stencil_attachment: pass_descriptor.depth_stencil_attachment.as_ref().map(|d| {
                Self::create_wgpu_depth_stencil_attachment_descriptor(
                    wgpu_resources,
                    global_render_resource_assignments,
                    d,
                    primary_swap_chain,
                    swap_chain_outputs,
                )
            }),
        })
    }

    fn get_texture_view<'a>(
        wgpu_resources: &'a WgpuResources,
        global_render_resource_assignments: &RenderResourceAssignments,
        primary_swap_chain: &Option<String>,
        swap_chain_outputs: &'a HashMap<String, wgpu::SwapChainOutput>,
        name: &str,
    ) -> &'a wgpu::TextureView {
        match name {
            resource_name::texture::SWAP_CHAIN => {
                if let Some(primary_swap_chain) = primary_swap_chain {
                    swap_chain_outputs
                        .get(primary_swap_chain)
                        .map(|output| &output.view)
                        .unwrap()
                } else {
                    panic!("No primary swap chain found for color attachment");
                }
            }
            _ => match global_render_resource_assignments.get(name) {
                Some(resource) => wgpu_resources.textures.get(&resource).unwrap(),
                None => {
                    if let Some(swap_chain_output) = swap_chain_outputs.get(name) {
                        &swap_chain_output.view
                    } else {
                        panic!("Color attachment {} does not exist", name);
                    }
                }
            },
        }
    }

    fn create_wgpu_color_attachment_descriptor<'a>(
        wgpu_resources: &'a WgpuResources,
        global_render_resource_assignments: &RenderResourceAssignments,
        color_attachment_descriptor: &RenderPassColorAttachmentDescriptor,
        primary_swap_chain: &Option<String>,
        swap_chain_outputs: &'a HashMap<String, wgpu::SwapChainOutput>,
    ) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
        let attachment = Self::get_texture_view(
            wgpu_resources,
            global_render_resource_assignments,
            primary_swap_chain,
            swap_chain_outputs,
            color_attachment_descriptor.attachment.as_str(),
        );

        let resolve_target = color_attachment_descriptor
            .resolve_target
            .as_ref()
            .map(|target| {
                Self::get_texture_view(
                    wgpu_resources,
                    global_render_resource_assignments,
                    primary_swap_chain,
                    swap_chain_outputs,
                    target.as_str(),
                )
            });

        wgpu::RenderPassColorAttachmentDescriptor {
            store_op: color_attachment_descriptor.store_op.wgpu_into(),
            load_op: color_attachment_descriptor.load_op.wgpu_into(),
            clear_color: color_attachment_descriptor.clear_color.wgpu_into(),
            attachment,
            resolve_target,
        }
    }

    fn create_wgpu_depth_stencil_attachment_descriptor<'a>(
        wgpu_resources: &'a WgpuResources,
        global_render_resource_assignments: &RenderResourceAssignments,
        depth_stencil_attachment_descriptor: &RenderPassDepthStencilAttachmentDescriptor,
        primary_swap_chain: &Option<String>,
        swap_chain_outputs: &'a HashMap<String, wgpu::SwapChainOutput>,
    ) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<'a> {
        let attachment = Self::get_texture_view(
            wgpu_resources,
            global_render_resource_assignments,
            primary_swap_chain,
            swap_chain_outputs,
            depth_stencil_attachment_descriptor.attachment.as_str(),
        );

        wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment,
            clear_depth: depth_stencil_attachment_descriptor.clear_depth,
            clear_stencil: depth_stencil_attachment_descriptor.clear_stencil,
            depth_load_op: depth_stencil_attachment_descriptor
                .depth_load_op
                .wgpu_into(),
            depth_store_op: depth_stencil_attachment_descriptor
                .depth_store_op
                .wgpu_into(),
            stencil_load_op: depth_stencil_attachment_descriptor
                .stencil_load_op
                .wgpu_into(),
            stencil_store_op: depth_stencil_attachment_descriptor
                .stencil_store_op
                .wgpu_into(),
        }
    }

    pub fn initialize_resource_providers(
        world: &mut World,
        resources: &mut Resources,
        render_context: &mut WgpuRenderContext<WgpuRenderResourceContext>,
    ) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.initialize(render_context, world, resources);
        }
    }

    fn parallel_resource_provider_update(
        world: &World,
        resources: &Resources,
        device: Arc<wgpu::Device>,
        global_wgpu_resources: &WgpuResources,
    ) -> (Vec::<wgpu::CommandBuffer>, Vec::<WgpuResources>) {
        let max_thread_count = 8;
        let (sender, receiver) = crossbeam_channel::bounded(max_thread_count);
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let chunk_size = (render_graph.resource_providers.len() + max_thread_count - 1) / max_thread_count; // divide ints rounding remainder up
        // println!("chunk {} {}", chunk_size, render_graph.resource_providers.len());
        let mut actual_thread_count = 0;
        crossbeam_utils::thread::scope(|s| {
            for resource_provider_chunk in render_graph.resource_providers.chunks_mut(chunk_size) {
                let device = device.clone();
                let resource_device = device.clone();
                let sender = sender.clone();
                let global_wgpu_resources = &*global_wgpu_resources;
                let world = &*world;
                let resources = &*resources;
                actual_thread_count += 1;
                // println!("spawn {}", resource_provider_chunk.len());
                s.spawn(move |_| {
                    let mut render_context = WgpuRenderContext::new(
                        device,
                        WgpuTransactionalRenderResourceContext::new(
                            resource_device,
                            global_wgpu_resources,
                        ),
                    );
                    for resource_provider in resource_provider_chunk.iter_mut() {
                        resource_provider.update(&mut render_context, world, resources);
                    }
                    sender.send(render_context.finish()).unwrap();
                });
            }
        }).unwrap();

        let mut command_buffers = Vec::new();
        let mut local_resources = Vec::new();
        for i in 0..actual_thread_count {
            let (command_buffer, render_resources) = receiver.recv().unwrap();
            if let Some(command_buffer) = command_buffer {
                command_buffers.push(command_buffer);
            }

            local_resources.push(render_resources.local_resources);

            // println!("got {}", i);
        }

        (command_buffers, local_resources)
    }

    pub fn update_resource_providers(
        world: &mut World,
        resources: &mut Resources,
        queue: &mut wgpu::Queue,
        device: Arc<wgpu::Device>,
        global_wgpu_resources: &mut WgpuResources,
    ) {
        let (mut command_buffers, local_resources) = Self::parallel_resource_provider_update(world, resources, device.clone(), global_wgpu_resources);
        for local_resource in local_resources {
            global_wgpu_resources.consume(local_resource);
        }

        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut results = Vec::new();
        let thread_count = 5;
        let chunk_size = (render_graph.resource_providers.len() + thread_count - 1) / thread_count; // divide ints rounding remainder up
        // crossbeam_utils::thread::scope(|s| {
        for resource_provider_chunk in render_graph.resource_providers.chunks_mut(chunk_size) {
            // TODO: try to unify this Device usage
            let device = device.clone();
            let resource_device = device.clone();
            // let sender = sender.clone();
            // s.spawn(|_| {
            // TODO: replace WgpuResources with Global+Local resources
            let mut render_context = WgpuRenderContext::new(
                device,
                WgpuTransactionalRenderResourceContext::new(resource_device, global_wgpu_resources),
            );
            for resource_provider in resource_provider_chunk.iter_mut() {
                resource_provider.finish_update(&mut render_context, world, resources);
            }
            results.push(render_context.finish());
            // sender.send(render_context.finish()).unwrap();
            // });
        }
        // });

        let mut local_resources = Vec::new();
        for (command_buffer, render_resources) in results {
            // for i in 0..thread_count {
            // let (command_buffer, wgpu_resources) = receiver.recv().unwrap();
            if let Some(command_buffer) = command_buffer {
                command_buffers.push(command_buffer);
            }

            local_resources.push(render_resources.local_resources);
            // println!("got {}", i);
        }
        for local_resource in local_resources {
            global_wgpu_resources.consume(local_resource);
        }

        queue.submit(&command_buffers);
    }

    pub fn create_queued_textures(&mut self, resources: &mut Resources) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();
        for (name, texture_descriptor) in render_graph.queued_textures.drain(..) {
            let resource = self.create_texture(&texture_descriptor, None);
            render_resource_assignments.set(&name, resource);
        }
    }

    pub fn handle_window_resized_events(
        resources: &mut Resources,
        device: &wgpu::Device,
        wgpu_resources: &mut WgpuResources,
        window_resized_event_reader: &mut EventReader<WindowResized>,
    ) {
        let windows = resources.get::<Windows>().unwrap();
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let mut handled_windows = HashSet::new();
        // iterate in reverse order so we can handle the latest window resize event first for each window.
        // we skip earlier events for the same window because it results in redundant work
        for window_resized_event in window_resized_events
            .iter(window_resized_event_reader)
            .rev()
        {
            if handled_windows.contains(&window_resized_event.id) {
                continue;
            }

            let window = windows
                .get(window_resized_event.id)
                .expect("Received window resized event for non-existent window");

            // TODO: consider making this a WgpuRenderContext method
            wgpu_resources.create_window_swap_chain(device, window);

            handled_windows.insert(window_resized_event.id);
        }
    }

    pub fn handle_window_created_events(
        resources: &mut Resources,
        device: &wgpu::Device,
        wgpu_resources: &mut WgpuResources,
        window_created_event_reader: &mut EventReader<WindowCreated>,
    ) {
        let windows = resources.get::<Windows>().unwrap();
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        for window_created_event in window_created_events.iter(window_created_event_reader) {
            let window = windows
                .get(window_created_event.id)
                .expect("Received window created event for non-existent window");
            #[cfg(feature = "bevy_winit")]
            {
                let winit_windows = resources.get::<bevy_winit::WinitWindows>().unwrap();
                let primary_winit_window = winit_windows.get_window(window.id).unwrap();
                let surface = wgpu::Surface::create(primary_winit_window.deref());
                wgpu_resources.set_window_surface(window.id, surface);
                wgpu_resources.create_window_swap_chain(device, window);
            }
        }
    }

    fn get_swap_chain_outputs(
        &mut self,
        resources: &Resources,
    ) -> (Option<String>, HashMap<String, wgpu::SwapChainOutput>) {
        let primary_window_id = resources
            .get::<Windows>()
            .unwrap()
            .get_primary()
            .map(|window| window.id);
        let primary_swap_chain =
            primary_window_id.map(|primary_window_id| primary_window_id.to_string());
        let swap_chain_outputs = self
            .global_context
            .render_resources
            .wgpu_resources
            .window_swap_chains
            .iter_mut()
            // TODO: include non-primary swap chains
            .filter(|(window_id, _swap_chain)| **window_id == primary_window_id.unwrap())
            .map(|(window_id, swap_chain)| {
                let swap_chain_texture = swap_chain
                    .get_next_texture()
                    .expect("Timeout when acquiring next swap chain texture");
                (window_id.to_string(), swap_chain_texture)
            })
            .collect::<HashMap<String, wgpu::SwapChainOutput>>();
        (primary_swap_chain, swap_chain_outputs)
    }
}

impl Renderer for WgpuRenderer {
    fn update(&mut self, world: &mut World, resources: &mut Resources) {
        Self::handle_window_created_events(
            resources,
            &self.global_context.device,
            &mut self.global_context.render_resources.wgpu_resources,
            &mut self.window_created_event_reader,
        );
        Self::handle_window_resized_events(
            resources,
            &self.global_context.device,
            &mut self.global_context.render_resources.wgpu_resources,
            &mut self.window_resized_event_reader,
        );
        if !self.intialized {
            Self::initialize_resource_providers(world, resources, &mut self.global_context);
            let buffer = self.global_context.finish_encoder();
            if let Some(buffer) = buffer {
                self.queue.submit(&[buffer]);
            }
            self.intialized = true;
        }

        Self::update_resource_providers(
            world,
            resources,
            &mut self.queue,
            self.global_context.device.clone(),
            &mut self.global_context.render_resources.wgpu_resources,
        );

        self.encoder = Some(
            self.global_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None }),
        );
        update_shader_assignments(world, resources, self);
        self.create_queued_textures(resources);
        let mut encoder = self.encoder.take().unwrap();

        render_resource_sets_system().run(world, resources);
        // setup draw targets
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.setup_pipeline_draw_targets(world, resources, self);

        let (primary_swap_chain, swap_chain_outputs) = self.get_swap_chain_outputs(resources);

        // begin render passes
        let pipeline_storage = resources.get::<AssetStorage<PipelineDescriptor>>().unwrap();
        let pipeline_compiler = resources.get::<PipelineCompiler>().unwrap();

        for (pass_name, pass_descriptor) in render_graph.pass_descriptors.iter() {
            let mut render_pass = {
                let global_render_resource_assignments =
                    resources.get::<RenderResourceAssignments>().unwrap();
                Self::create_render_pass(
                    &self.global_context.render_resources.wgpu_resources,
                    pass_descriptor,
                    &global_render_resource_assignments,
                    &mut encoder,
                    &primary_swap_chain,
                    &swap_chain_outputs,
                )
            };
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pass_pipeline)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let pipeline_descriptor =
                                pipeline_storage.get(compiled_pipeline_handle).unwrap();
                            let render_pipeline =
                                self.render_pipelines.get(compiled_pipeline_handle).unwrap();
                            render_pass.set_pipeline(render_pipeline);

                            let mut wgpu_render_pass = WgpuRenderPass {
                                render_pass: &mut render_pass,
                                pipeline_descriptor,
                                wgpu_resources: &self
                                    .global_context
                                    .render_resources
                                    .wgpu_resources,
                                renderer: &self,
                                bound_bind_groups: HashMap::default(),
                            };

                            for draw_target_name in pipeline_descriptor.draw_targets.iter() {
                                let draw_target =
                                    render_graph.draw_targets.get(draw_target_name).unwrap();
                                draw_target.draw(
                                    world,
                                    resources,
                                    &mut wgpu_render_pass,
                                    *compiled_pipeline_handle,
                                );
                            }
                        }
                    }
                }
            }
        }

        let command_buffer = encoder.finish();
        self.queue.submit(&[command_buffer]);
    }

    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.global_context
            .render_resources
            .wgpu_resources
            .create_buffer_with_data(&self.global_context.device, buffer_info, data)
    }

    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource {
        self.global_context
            .render_resources
            .wgpu_resources
            .create_buffer(&self.global_context.device, buffer_info)
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.global_context
            .render_resources
            .wgpu_resources
            .resource_info
            .get(&resource)
    }

    fn get_resource_info_mut(&mut self, resource: RenderResource) -> Option<&mut ResourceInfo> {
        self.global_context
            .render_resources
            .wgpu_resources
            .resource_info
            .get_mut(&resource)
    }

    fn remove_buffer(&mut self, resource: RenderResource) {
        self.global_context
            .render_resources
            .wgpu_resources
            .remove_buffer(resource);
    }

    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn Renderer),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped(&buffer_info, self, setup_data);
        self.global_context
            .render_resources
            .wgpu_resources
            .assign_buffer(buffer, buffer_info)
    }

    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        self.global_context
            .render_resources
            .wgpu_resources
            .copy_buffer_to_buffer(
                self.encoder.as_mut().unwrap(),
                source_buffer,
                source_offset,
                destination_buffer,
                destination_offset,
                size,
            );
    }

    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.global_context
            .render_resources
            .wgpu_resources
            .create_sampler(&self.global_context.device, sampler_descriptor)
    }

    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource {
        if let Some(bytes) = bytes {
            self.global_context
                .render_resources
                .wgpu_resources
                .create_texture_with_data(
                    &self.global_context.device,
                    self.encoder.as_mut().unwrap(),
                    texture_descriptor,
                    bytes,
                )
        } else {
            self.global_context
                .render_resources
                .wgpu_resources
                .create_texture(&self.global_context.device, texture_descriptor)
        }
    }

    fn remove_texture(&mut self, resource: RenderResource) {
        self.global_context
            .render_resources
            .wgpu_resources
            .remove_texture(resource);
    }

    fn remove_sampler(&mut self, resource: RenderResource) {
        self.global_context
            .render_resources
            .wgpu_resources
            .remove_sampler(resource);
    }

    fn get_render_resources(&self) -> &AssetResources {
        &self
            .global_context
            .render_resources
            .wgpu_resources
            .asset_resources
    }

    fn get_render_resources_mut(&mut self) -> &mut AssetResources {
        &mut self
            .global_context
            .render_resources
            .wgpu_resources
            .asset_resources
    }

    fn setup_bind_groups(
        &mut self,
        render_resource_assignments: &RenderResourceAssignments,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            if let Some((render_resource_set_id, _indices)) =
                render_resource_assignments.get_render_resource_set_id(bind_group.id)
            {
                if let None = self
                    .global_context
                    .render_resources
                    .wgpu_resources
                    .get_bind_group(bind_group.id, *render_resource_set_id)
                {
                    self.global_context
                        .render_resources
                        .wgpu_resources
                        .create_bind_group(
                            &self.global_context.device,
                            bind_group,
                            render_resource_assignments,
                        );
                } else {
                    log::trace!(
                        "reusing RenderResourceSet {:?} for bind group {}",
                        render_resource_set_id,
                        bind_group.index
                    );
                }
            }
        }
    }

    fn setup_render_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &mut PipelineDescriptor,
        shader_storage: &AssetStorage<Shader>,
    ) {
        if self.render_pipelines.contains_key(&pipeline_handle) {
            return;
        }

        let layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in layout.bind_groups.iter() {
            if let None = self
                .global_context
                .render_resources
                .wgpu_resources
                .bind_group_layouts
                .get(&bind_group.id)
            {
                let bind_group_layout_binding = bind_group
                    .bindings
                    .iter()
                    .map(|binding| wgpu::BindGroupLayoutEntry {
                        binding: binding.index,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: (&binding.bind_type).wgpu_into(),
                    })
                    .collect::<Vec<wgpu::BindGroupLayoutEntry>>();
                let wgpu_bind_group_layout = self.global_context.device.create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        bindings: bind_group_layout_binding.as_slice(),
                        label: None,
                    },
                );

                self.global_context
                    .render_resources
                    .wgpu_resources
                    .bind_group_layouts
                    .insert(bind_group.id, wgpu_bind_group_layout);
            }
        }

        // setup and collect bind group layouts
        let bind_group_layouts = layout
            .bind_groups
            .iter()
            .map(|bind_group| {
                self.global_context
                    .render_resources
                    .wgpu_resources
                    .bind_group_layouts
                    .get(&bind_group.id)
                    .unwrap()
            })
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout =
            self.global_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: bind_group_layouts.as_slice(),
                });

        let owned_vertex_buffer_descriptors = layout
            .vertex_buffer_descriptors
            .iter()
            .map(|v| v.wgpu_into())
            .collect::<Vec<OwnedWgpuVertexBufferDescriptor>>();

        let color_states = pipeline_descriptor
            .color_states
            .iter()
            .map(|c| c.wgpu_into())
            .collect::<Vec<wgpu::ColorStateDescriptor>>();

        if let None = self
            .global_context
            .render_resources
            .wgpu_resources
            .shader_modules
            .get(&pipeline_descriptor.shader_stages.vertex)
        {
            self.global_context
                .render_resources
                .wgpu_resources
                .create_shader_module(
                    &self.global_context.device,
                    pipeline_descriptor.shader_stages.vertex,
                    shader_storage,
                );
        }

        if let Some(fragment_handle) = pipeline_descriptor.shader_stages.fragment {
            if let None = self
                .global_context
                .render_resources
                .wgpu_resources
                .shader_modules
                .get(&fragment_handle)
            {
                self.global_context
                    .render_resources
                    .wgpu_resources
                    .create_shader_module(
                        &self.global_context.device,
                        fragment_handle,
                        shader_storage,
                    );
            }
        };

        let vertex_shader_module = self
            .global_context
            .render_resources
            .wgpu_resources
            .shader_modules
            .get(&pipeline_descriptor.shader_stages.vertex)
            .unwrap();

        let fragment_shader_module = match pipeline_descriptor.shader_stages.fragment {
            Some(fragment_handle) => Some(
                self.global_context
                    .render_resources
                    .wgpu_resources
                    .shader_modules
                    .get(&fragment_handle)
                    .unwrap(),
            ),
            None => None,
        };

        let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vertex_shader_module,
                entry_point: "main",
            },
            fragment_stage: match pipeline_descriptor.shader_stages.fragment {
                Some(_) => Some(wgpu::ProgrammableStageDescriptor {
                    entry_point: "main",
                    module: fragment_shader_module.as_ref().unwrap(),
                }),
                None => None,
            },
            rasterization_state: pipeline_descriptor
                .rasterization_state
                .as_ref()
                .map(|r| r.wgpu_into()),
            primitive_topology: pipeline_descriptor.primitive_topology.wgpu_into(),
            color_states: &color_states,
            depth_stencil_state: pipeline_descriptor
                .depth_stencil_state
                .as_ref()
                .map(|d| d.wgpu_into()),
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: pipeline_descriptor.index_format.wgpu_into(),
                vertex_buffers: &owned_vertex_buffer_descriptors
                    .iter()
                    .map(|v| v.into())
                    .collect::<Vec<wgpu::VertexBufferDescriptor>>(),
            },
            sample_count: pipeline_descriptor.sample_count,
            sample_mask: pipeline_descriptor.sample_mask,
            alpha_to_coverage_enabled: pipeline_descriptor.alpha_to_coverage_enabled,
        };

        let render_pipeline = self
            .global_context
            .device
            .create_render_pipeline(&mut render_pipeline_descriptor);
        self.render_pipelines
            .insert(pipeline_handle, render_pipeline);
    }
}
