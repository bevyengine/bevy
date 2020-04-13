use super::WgpuResources;
use crate::renderer_2::{
    render_resource_sets_system, WgpuRenderContext, WgpuRenderResourceContext,
    WgpuTransactionalRenderResourceContext,
};
use bevy_app::{EventReader, Events};
use bevy_asset::AssetStorage;
use bevy_render::{
    pipeline::{update_shader_assignments, PipelineCompiler, PipelineDescriptor},
    render_graph::RenderGraph,
    render_resource::RenderResourceAssignments,
    renderer_2::{RenderContext, RenderResourceContext},
};
use bevy_window::{WindowCreated, WindowResized, Windows};
use legion::prelude::*;
use std::{collections::HashSet, ops::Deref, sync::Arc};

pub struct WgpuRenderer {
    pub global_context: WgpuRenderContext<WgpuRenderResourceContext>,
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
            global_context: WgpuRenderContext::new(
                device.clone(),
                WgpuRenderResourceContext::new(device),
            ),
            queue,
            window_resized_event_reader,
            window_created_event_reader,
            intialized: false,
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
    ) -> (Vec<wgpu::CommandBuffer>, Vec<WgpuResources>) {
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
        })
        .unwrap();

        let mut command_buffers = Vec::new();
        let mut local_resources = Vec::new();
        for _i in 0..actual_thread_count {
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
        let (mut command_buffers, local_resources) = Self::parallel_resource_provider_update(
            world,
            resources,
            device.clone(),
            global_wgpu_resources,
        );
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
            let resource = self
                .global_context
                .resources_mut()
                .create_texture(&texture_descriptor);
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

    pub fn update(&mut self, world: &mut World, resources: &mut Resources) {
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

        update_shader_assignments(world, resources, &self.global_context);
        self.create_queued_textures(resources);

        render_resource_sets_system().run(world, resources);
        // setup draw targets
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.setup_pipeline_draw_targets(world, resources, &mut self.global_context);

        // get next swap chain texture on primary window
        let primary_window_id = resources
            .get::<Windows>()
            .unwrap()
            .get_primary()
            .map(|window| window.id);
        if let Some(primary_window_id) = primary_window_id {
            self.global_context
                .render_resources
                .next_swap_chain_texture(primary_window_id);
            self.global_context.primary_window = Some(primary_window_id);
        }

        // begin render passes
        let pipeline_storage = resources.get::<AssetStorage<PipelineDescriptor>>().unwrap();
        let pipeline_compiler = resources.get::<PipelineCompiler>().unwrap();

        for (pass_name, pass_descriptor) in render_graph.pass_descriptors.iter() {
            let global_render_resource_assignments =
                resources.get::<RenderResourceAssignments>().unwrap();
            self.global_context.begin_pass(
                pass_descriptor,
                &global_render_resource_assignments,
                &mut |render_pass| {
                    if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                        for pass_pipeline in pass_pipelines.iter() {
                            if let Some(compiled_pipelines_iter) =
                                pipeline_compiler.iter_compiled_pipelines(*pass_pipeline)
                            {
                                for compiled_pipeline_handle in compiled_pipelines_iter {
                                    let pipeline_descriptor =
                                        pipeline_storage.get(compiled_pipeline_handle).unwrap();
                                    render_pass.set_pipeline(*compiled_pipeline_handle);

                                    for draw_target_name in pipeline_descriptor.draw_targets.iter()
                                    {
                                        let draw_target = render_graph
                                            .draw_targets
                                            .get(draw_target_name)
                                            .unwrap();
                                        draw_target.draw(
                                            world,
                                            resources,
                                            render_pass,
                                            *compiled_pipeline_handle,
                                            pipeline_descriptor,
                                        );
                                    }
                                }
                            }
                        }
                    }
                },
            );
        }

        let command_buffer = self.global_context.finish_encoder();
        if let Some(command_buffer) = command_buffer {
            self.queue.submit(&[command_buffer]);
        }

        // clear primary swap chain texture
        if let Some(primary_window_id) = primary_window_id {
            self.global_context
                .render_resources
                .drop_swap_chain_texture(primary_window_id);
        }
    }
}
