mod graph_runner;
mod render_device;

use bevy_utils::tracing::info;
pub use graph_runner::*;
pub use render_device::*;

use crate::render_graph::RenderGraph;
use bevy_ecs::prelude::*;
use std::sync::Arc;
use wgpu::{BackendBit, CommandEncoder, DeviceDescriptor, Instance, Queue, RequestAdapterOptions};

pub fn render_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });
    let graph = world.get_resource::<RenderGraph>().unwrap();
    let render_device = world.get_resource::<RenderDevice>().unwrap();
    let render_queue = world.get_resource::<RenderQueue>().unwrap();
    RenderGraphRunner::run(
        graph,
        render_device.clone(), // TODO: is this clone really necessary?
        render_queue,
        world,
    )
    .unwrap();
}

pub type RenderQueue = Arc<Queue>;
pub type RenderInstance = Instance;

pub async fn initialize_renderer(
    backends: BackendBit,
    request_adapter_options: &RequestAdapterOptions<'_>,
    device_descriptor: &DeviceDescriptor<'_>,
) -> (RenderInstance, RenderDevice, RenderQueue) {
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
    (instance, RenderDevice::from(device), queue)
}

pub struct RenderContext {
    pub render_device: RenderDevice,
    pub command_encoder: CommandEncoder,
}
