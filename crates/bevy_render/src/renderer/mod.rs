mod graph_runner;
mod render_device;

use bevy_utils::tracing::{info, info_span};
pub use graph_runner::*;
pub use render_device::*;

use crate::{
    render_graph::RenderGraph,
    view::{ExtractedWindows, ViewTarget},
};
use bevy_ecs::prelude::*;
use std::sync::Arc;
use wgpu::{CommandEncoder, DeviceDescriptor, Instance, Queue, RequestAdapterOptions};

/// Updates the [`RenderGraph`] with all of its nodes and then runs it to render the entire frame.
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
    {
        let span = info_span!("present_frames");
        let _guard = span.enter();

        // Remove ViewTarget components to ensure swap chain TextureViews are dropped.
        // If all TextureViews aren't dropped before present, acquiring the next swap chain texture will fail.
        let view_entities = world
            .query_filtered::<Entity, With<ViewTarget>>()
            .iter(world)
            .collect::<Vec<_>>();
        for view_entity in view_entities {
            world.entity_mut(view_entity).remove::<ViewTarget>();
        }

        let mut windows = world.get_resource_mut::<ExtractedWindows>().unwrap();
        for window in windows.values_mut() {
            if let Some(texture_view) = window.swap_chain_texture.take() {
                if let Some(surface_texture) = texture_view.take_surface_texture() {
                    surface_texture.present();
                }
            }
        }
    }
}

/// This queue is used to enqueue tasks for the GPU to execute asynchronously.
pub type RenderQueue = Arc<Queue>;

/// The GPU instance is used to initialize the [`RenderQueue`] and [`RenderDevice`],
/// aswell as to create [`WindowSurfaces`](crate::view::window::WindowSurfaces).
pub type RenderInstance = Instance;

/// Initializes the renderer by retrieving and preparing the GPU instance, device and queue
/// for the specified backend.
pub async fn initialize_renderer(
    instance: &Instance,
    request_adapter_options: &RequestAdapterOptions<'_>,
    device_descriptor: &DeviceDescriptor<'_>,
) -> (RenderDevice, RenderQueue) {
    let adapter = instance
        .request_adapter(request_adapter_options)
        .await
        .expect("Unable to find a GPU! Make sure you have installed required drivers!");

    info!("{:?}", adapter.get_info());

    #[cfg(feature = "wgpu_trace")]
    let trace_path = {
        let path = std::path::Path::new("wgpu_trace");
        // ignore potential error, wgpu will log it
        let _ = std::fs::create_dir(path);
        Some(path)
    };
    #[cfg(not(feature = "wgpu_trace"))]
    let trace_path = None;

    let (device, queue) = adapter
        .request_device(device_descriptor, trace_path)
        .await
        .unwrap();
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    (RenderDevice::from(device), queue)
}

/// The context with all information required to interact with the GPU.
///
/// The [`RenderDevice`] is used to create render resources and the
/// the [`CommandEncoder`] is used to record a series of GPU operations.
pub struct RenderContext {
    pub render_device: RenderDevice,
    pub command_encoder: CommandEncoder,
}
