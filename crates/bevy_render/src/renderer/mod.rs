mod gpu_device;
mod graph_runner;

use bevy_utils::tracing::{error, info_span};
pub use gpu_device::*;
pub use graph_runner::*;

use crate::{
    render_graph::RenderGraph,
    view::{ExtractedWindows, ViewTarget},
};
use bevy_ecs::prelude::*;
use bevy_gpu::{GpuDevice, GpuQueue};
use bevy_time::TimeSender;
use bevy_utils::Instant;

/// Updates the [`RenderGraph`] with all of its nodes and then runs it to render the entire frame.
pub fn render_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });
    let graph = world.resource::<RenderGraph>();
    let gpu_device = world.resource::<GpuDevice>();
    let gpu_queue = world.resource::<GpuQueue>();

    if let Err(e) = RenderGraphRunner::run(
        graph,
        gpu_device.clone(), // TODO: is this clone really necessary?
        gpu_queue,
        world,
    ) {
        error!("Error running render graph:");
        {
            let mut src: &dyn std::error::Error = &e;
            loop {
                error!("> {}", src);
                match src.source() {
                    Some(s) => src = s,
                    None => break,
                }
            }
        }

        panic!("Error running render graph: {e}");
    }

    {
        let _span = info_span!("present_frames").entered();

        // Remove ViewTarget components to ensure swap chain TextureViews are dropped.
        // If all TextureViews aren't dropped before present, acquiring the next swap chain texture will fail.
        let view_entities = world
            .query_filtered::<Entity, With<ViewTarget>>()
            .iter(world)
            .collect::<Vec<_>>();
        for view_entity in view_entities {
            world.entity_mut(view_entity).remove::<ViewTarget>();
        }

        let mut windows = world.resource_mut::<ExtractedWindows>();
        for window in windows.values_mut() {
            if let Some(texture_view) = window.swap_chain_texture.take() {
                if let Some(surface_texture) = texture_view.take_surface_texture() {
                    surface_texture.present();
                }
            }
        }

        #[cfg(feature = "tracing-tracy")]
        bevy_utils::tracing::event!(
            bevy_utils::tracing::Level::INFO,
            message = "finished frame",
            tracy.frame_mark = true
        );
    }

    // update the time and send it to the app world
    let time_sender = world.resource::<TimeSender>();
    time_sender.0.try_send(Instant::now()).expect(
        "The TimeSender channel should always be empty during render. You might need to add the bevy::core::time_system to your app.",
    );
}
