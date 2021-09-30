mod gpu_device;
mod graph_runner;

pub use gpu_device::*;
pub use graph_runner::*;

use crate::render_graph::RenderGraph;
use bevy_ecs::prelude::*;
use bevy_utils::tracing::info;
use std::sync::Arc;
use wgpu::{BackendBit, CommandEncoder, DeviceDescriptor, Instance, Queue, RequestAdapterOptions};

pub type GpuQueue = Arc<Queue>;
pub type GpuInstance = Instance;

pub struct GpuContext {
    pub gpu_device: GpuDevice,
    pub command_encoder: CommandEncoder,
}

pub async fn initialize_renderer(
    backends: BackendBit,
    request_adapter_options: &RequestAdapterOptions<'_>,
    device_descriptor: &DeviceDescriptor<'_>,
) -> (GpuInstance, GpuDevice, GpuQueue) {
    let instance = wgpu::Instance::new(backends);

    let adapter = instance
        .request_adapter(request_adapter_options)
        .await
        .expect("Unable to find a GPU! Make sure you have installed required drivers!");

    #[cfg(not(target_arch = "wasm32"))]
    info!("{:?}", adapter.get_info());

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
        .request_device(device_descriptor, trace_path)
        .await
        .unwrap();
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    (instance, GpuDevice::from(device), queue)
}

pub fn render_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });
    let graph = world.get_resource::<RenderGraph>().unwrap();
    let gpu_device = world.get_resource::<GpuDevice>().unwrap();
    let gpu_queue = world.get_resource::<GpuQueue>().unwrap();
    RenderGraphRunner::run(
        graph,
        gpu_device.clone(), // TODO: is this clone really necessary?
        gpu_queue,
        world,
    )
    .unwrap();
}
