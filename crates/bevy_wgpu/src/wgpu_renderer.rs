use crate::{
    renderer::{WgpuRenderGraphExecutor, WgpuRenderResourceContext},
    wgpu_type_converter::WgpuInto,
    WgpuBackend, WgpuOptions, WgpuPowerOptions,
};
use bevy_app::{Events, ManualEventReader};
use bevy_ecs::world::{Mut, World};
use bevy_render::{
    render_graph::{DependentNodeStager, RenderGraph, RenderGraphStager},
    renderer::RenderResourceContext,
};
use bevy_window::{WindowCreated, WindowResized, Windows};
use std::{ops::Deref, sync::Arc};

pub struct WgpuRenderer {
    pub instance: wgpu::Instance,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub window_resized_event_reader: ManualEventReader<WindowResized>,
    pub window_created_event_reader: ManualEventReader<WindowCreated>,
    pub initialized: bool,
}

impl WgpuRenderer {
    pub async fn new(options: WgpuOptions) -> Self {
        let backend = match options.backend {
            WgpuBackend::Auto => wgpu::Backends::PRIMARY,
            WgpuBackend::Vulkan => wgpu::Backends::VULKAN,
            WgpuBackend::Metal => wgpu::Backends::METAL,
            WgpuBackend::Dx12 => wgpu::Backends::DX12,
            WgpuBackend::Dx11 => wgpu::Backends::DX11,
            WgpuBackend::Gl => wgpu::Backends::GL,
            WgpuBackend::BrowserWgpu => wgpu::Backends::BROWSER_WEBGPU,
        };
        let instance = wgpu::Instance::new(backend);

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: match options.power_pref {
                    WgpuPowerOptions::HighPerformance => wgpu::PowerPreference::HighPerformance,
                    WgpuPowerOptions::Adaptive => wgpu::PowerPreference::LowPower,
                    WgpuPowerOptions::LowPower => wgpu::PowerPreference::LowPower,
                },
                compatible_surface: None,
            })
            .await
            .expect("Unable to find a GPU! Make sure you have installed required drivers!");

        #[cfg(feature = "trace")]
        let trace_path = {
            let path = std::path::Path::new("wgpu_trace");
            // ignore potential error, wgpu will log it
            let _ = std::fs::create_dir(path);
            Some(path)
        };
        #[cfg(not(feature = "trace"))]
        let trace_path = None;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: options.device_label.as_ref().map(|a| a.as_ref()),
                    features: options.features.wgpu_into(),
                    limits: options.limits.wgpu_into(),
                },
                trace_path,
            )
            .await
            .unwrap();
        let device = Arc::new(device);
        WgpuRenderer {
            instance,
            device,
            queue,
            window_resized_event_reader: Default::default(),
            window_created_event_reader: Default::default(),
            initialized: false,
        }
    }

    pub fn handle_window_created_events(&mut self, world: &mut World) {
        let world = world.cell();
        let mut render_resource_context = world
            .get_resource_mut::<Box<dyn RenderResourceContext>>()
            .unwrap();
        let render_resource_context = render_resource_context
            .downcast_mut::<WgpuRenderResourceContext>()
            .unwrap();
        let windows = world.get_resource::<Windows>().unwrap();
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();
        for window_created_event in self
            .window_created_event_reader
            .iter(&window_created_events)
        {
            let window = windows
                .get(window_created_event.id)
                .expect("Received window created event for non-existent window.");
            #[cfg(feature = "bevy_winit")]
            {
                let winit_windows = world.get_resource::<bevy_winit::WinitWindows>().unwrap();
                let winit_window = winit_windows.get_window(window.id()).unwrap();
                // SAFE: The raw window handle created from a `winit::Window` is always valid.
                let surface = unsafe { self.instance.create_surface(winit_window.deref()) };
                render_resource_context.set_window_surface(window.id(), surface);
            }
        }
    }

    pub fn run_graph(&mut self, world: &mut World) {
        world.resource_scope(|world, mut render_graph: Mut<RenderGraph>| {
            render_graph.prepare(world);
            // stage nodes
            let mut stager = DependentNodeStager::loose_grouping();
            let stages = stager.get_stages(&render_graph).unwrap();
            let mut borrowed = stages.borrow(&mut render_graph);

            // execute stages
            let graph_executor = WgpuRenderGraphExecutor {
                max_thread_count: 2,
            };
            graph_executor.execute(world, self.device.clone(), &mut self.queue, &mut borrowed);
        })
    }

    pub fn update(&mut self, world: &mut World) {
        self.handle_window_created_events(world);
        self.run_graph(world);

        let render_resource_context = world
            .get_resource::<Box<dyn RenderResourceContext>>()
            .unwrap();
        render_resource_context.drop_all_swap_chain_textures();
        render_resource_context.remove_stale_bind_groups();
    }
}
