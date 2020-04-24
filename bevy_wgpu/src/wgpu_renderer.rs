use crate::renderer_2::{
    render_resource_sets_system, WgpuRenderContext, WgpuRenderResourceContext, WgpuRenderGraphExecutor,
};
use bevy_app::{EventReader, Events};
use bevy_render::{
    pipeline::{update_shader_assignments},
    render_graph::RenderGraph,
    render_graph_2::{RenderGraph2, RenderGraphStager, DependentNodeStager},
    render_resource::RenderResourceAssignments,
    renderer_2::{GlobalRenderResourceContext, RenderResourceContext},
};
use bevy_window::{WindowCreated, WindowResized, Windows};
use legion::prelude::*;
use std::{ops::Deref, sync::Arc};
pub struct WgpuRenderer {
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
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
            device,
            queue,
            window_resized_event_reader,
            window_created_event_reader,
            intialized: false,
        }
    }

    pub fn initialize_resource_providers(
        world: &mut World,
        resources: &Resources,
        render_context: &mut WgpuRenderContext,
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
        render_resource_context: &WgpuRenderResourceContext,
    ) -> Vec<wgpu::CommandBuffer> {
        let max_thread_count = 8;
        let (sender, receiver) = crossbeam_channel::bounded(max_thread_count);
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let chunk_size =
            (render_graph.resource_providers.len() + max_thread_count - 1) / max_thread_count; // divide ints rounding remainder up
                                                                                               // println!("chunk {} {}", chunk_size, render_graph.resource_providers.len());
        let mut actual_thread_count = 0;
        crossbeam_utils::thread::scope(|s| {
            for resource_provider_chunk in render_graph.resource_providers.chunks_mut(chunk_size) {
                let device = device.clone();
                let sender = sender.clone();
                let world = &*world;
                let resources = &*resources;
                actual_thread_count += 1;
                let render_resource_context = render_resource_context.clone();
                s.spawn(move |_| {
                    let mut render_context =
                        WgpuRenderContext::new(device, render_resource_context);
                    for resource_provider in resource_provider_chunk.iter_mut() {
                        resource_provider.update(&mut render_context, world, resources);
                    }
                    sender.send(render_context.finish()).unwrap();
                });
            }
        })
        .unwrap();

        let mut command_buffers = Vec::new();
        for _i in 0..actual_thread_count {
            let command_buffer = receiver.recv().unwrap();
            if let Some(command_buffer) = command_buffer {
                command_buffers.push(command_buffer);
            }
        }

        command_buffers
    }

    pub fn update_resource_providers(
        &mut self,
        world: &mut World,
        resources: &Resources,
        render_resource_context: &WgpuRenderResourceContext,
    ) {
        let mut command_buffers = Self::parallel_resource_provider_update(
            world,
            resources,
            self.device.clone(),
            render_resource_context,
        );

        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut results = Vec::new();
        let thread_count = 5;
        let chunk_size = (render_graph.resource_providers.len() + thread_count - 1) / thread_count; // divide ints rounding remainder up
                                                                                                    // crossbeam_utils::thread::scope(|s| {
        for resource_provider_chunk in render_graph.resource_providers.chunks_mut(chunk_size) {
            // TODO: try to unify this Device usage
            let device = self.device.clone();
            // let sender = sender.clone();
            // s.spawn(|_| {
            // TODO: replace WgpuResources with Global+Local resources
            let mut render_context =
                WgpuRenderContext::new(device, render_resource_context.clone());
            for resource_provider in resource_provider_chunk.iter_mut() {
                resource_provider.finish_update(&mut render_context, world, resources);
            }
            results.push(render_context.finish());
            // sender.send(render_context.finish()).unwrap();
            // });
        }
        // });

        for command_buffer in results {
            // for i in 0..thread_count {
            // let (command_buffer, wgpu_resources) = receiver.recv().unwrap();
            if let Some(command_buffer) = command_buffer {
                command_buffers.push(command_buffer);
            }

            // println!("got {}", i);
        }

        self.queue.submit(&command_buffers);
    }

    pub fn create_queued_textures(
        &mut self,
        resources: &Resources,
        global_render_resources: &mut WgpuRenderResourceContext,
    ) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();
        for (name, texture_descriptor) in render_graph.queued_textures.drain(..) {
            let resource = global_render_resources.create_texture(&texture_descriptor);
            render_resource_assignments.set(&name, resource);
        }
    }

    pub fn handle_window_created_events(
        &mut self,
        resources: &Resources,
        global_render_resource_context: &mut WgpuRenderResourceContext,
    ) {
        let windows = resources.get::<Windows>().unwrap();
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        for window_created_event in
            window_created_events.iter(&mut self.window_created_event_reader)
        {
            let window = windows
                .get(window_created_event.id)
                .expect("Received window created event for non-existent window");
            #[cfg(feature = "bevy_winit")]
            {
                let winit_windows = resources.get::<bevy_winit::WinitWindows>().unwrap();
                let winit_window = winit_windows.get_window(window.id).unwrap();
                let surface = wgpu::Surface::create(winit_window.deref());
                global_render_resource_context
                    .wgpu_resources
                    .set_window_surface(window.id, surface);
            }
        }
    }

    pub fn run_graph(&mut self, world: &mut World, resources: &mut Resources) {
        // run systems
        let mut executor = {
            let mut render_graph = resources.get_mut::<RenderGraph2>().unwrap();
            render_graph.take_executor()
        };

        if let Some(executor) = executor.as_mut() {
            executor.execute(world, resources);
        }

        let mut render_graph = resources.get_mut::<RenderGraph2>().unwrap();
        if let Some(executor) = executor.take() {
            render_graph.set_executor(executor);
        }

        // stage nodes
        let mut stager = DependentNodeStager::loose_grouping();
        let stages = stager.get_stages(&render_graph).unwrap();
        let mut borrowed = stages.borrow(&mut render_graph);

        // execute stages
        let executor = WgpuRenderGraphExecutor {
            max_thread_count: 2,
        };
        executor.execute(world, resources, self.device.clone(), &mut self.queue, &mut borrowed);
    }

    pub fn update(&mut self, world: &mut World, resources: &mut Resources) {
        let mut encoder = {
            let mut global_context = resources.get_mut::<GlobalRenderResourceContext>().unwrap();
            let render_resource_context = global_context
                .context
                .downcast_mut::<WgpuRenderResourceContext>()
                .unwrap();

            self.handle_window_created_events(resources, render_resource_context);

            let mut render_context =
                WgpuRenderContext::new(self.device.clone(), render_resource_context.clone());
            if !self.intialized {
                Self::initialize_resource_providers(world, resources, &mut render_context);
                let buffer = render_context.finish();
                if let Some(buffer) = buffer {
                    self.queue.submit(&[buffer]);
                }
                self.intialized = true;
            }

            self.update_resource_providers(world, resources, render_resource_context);

            update_shader_assignments(world, resources, &render_context);
            self.create_queued_textures(resources, &mut render_context.render_resources);
            render_context.command_encoder.take()
        };

        self.run_graph(world, resources);
        render_resource_sets_system().run(world, resources);
        // TODO: add to POST_UPDATE and remove redundant global_context
        let mut global_context = resources.get_mut::<GlobalRenderResourceContext>().unwrap();
        let render_resource_context = global_context
            .context
            .downcast_mut::<WgpuRenderResourceContext>()
            .unwrap();

        let mut render_context =
            WgpuRenderContext::new(self.device.clone(), render_resource_context.clone());
        if let Some(command_encoder) = encoder.take() {
            render_context.command_encoder.set(command_encoder);
        }

        // setup draw targets
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.setup_pipeline_draw_targets(world, resources, &mut render_context);

        let command_buffer = render_context.finish();
        if let Some(command_buffer) = command_buffer {
            self.queue.submit(&[command_buffer]);
        }

        // clear swap chain textures
        render_context
            .render_resources
            .drop_all_swap_chain_textures();
    }
}
