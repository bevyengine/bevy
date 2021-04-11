use crate::{
    render_graph_executor::WgpuRenderGraphExecutor, type_converter::WgpuInto, WgpuBackend,
    WgpuOptions, WgpuPowerOptions, WgpuRenderResourceContext,
};
use bevy_ecs::world::{Mut, World};
use bevy_render2::{
    render_graph::{DependentNodeStager, ExtractedWindows, RenderGraph, RenderGraphStager},
    renderer::RenderResources,
};
use std::sync::Arc;

pub struct WgpuRenderer {
    pub instance: wgpu::Instance,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub initialized: bool,
}

impl WgpuRenderer {
    pub async fn new(options: WgpuOptions) -> Self {
        let backend = match options.backend {
            WgpuBackend::Auto => wgpu::BackendBit::PRIMARY,
            WgpuBackend::Vulkan => wgpu::BackendBit::VULKAN,
            WgpuBackend::Metal => wgpu::BackendBit::METAL,
            WgpuBackend::Dx12 => wgpu::BackendBit::DX12,
            WgpuBackend::Dx11 => wgpu::BackendBit::DX11,
            WgpuBackend::Gl => wgpu::BackendBit::GL,
            WgpuBackend::BrowserWgpu => wgpu::BackendBit::BROWSER_WEBGPU,
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
            initialized: false,
        }
    }

    pub fn handle_new_windows(&mut self, world: &mut World) {
        let world = world.cell();
        let mut render_resources = world.get_resource_mut::<RenderResources>().unwrap();
        let render_resource_context = render_resources
            .downcast_mut::<WgpuRenderResourceContext>()
            .unwrap();
        let extracted_windows = world.get_resource::<ExtractedWindows>().unwrap();
        for (id, window) in extracted_windows.iter() {
            if !render_resource_context.contains_window_surface(*id) {
                let surface = unsafe { self.instance.create_surface(&window.handle) };
                render_resource_context.set_window_surface(*id, surface);
            }
        }
    }

    pub fn run_graph(&mut self, world: &mut World) {
        world.resource_scope(|world, mut render_graph: Mut<RenderGraph>| {
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
        self.handle_new_windows(world);
        self.run_graph(world);

        let render_resources = world.get_resource::<RenderResources>().unwrap();
        render_resources.drop_all_swap_chain_textures();
        render_resources.remove_stale_bind_groups();
    }
}
