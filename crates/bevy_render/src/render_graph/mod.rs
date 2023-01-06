mod context;
mod edge;
mod graph;
mod graph_runner;
mod node;
mod node_slot;

pub use context::*;
pub use edge::*;
pub use graph::*;
pub use node::*;
pub use node_slot::*;

use crate::render_graph::graph_runner::RenderGraphRunner;
use crate::view::{ExtractedWindows, ViewTarget};
use bevy_ecs::prelude::*;
use bevy_gpu::{CommandEncoder, Device, Queue};
use bevy_log::error;
use bevy_time::TimeSender;
use bevy_utils::tracing::info_span;
use bevy_utils::Instant;
use thiserror::Error;

/// The context with all information required to interact with the GPU during the [`RenderStage::Render`](crate::RenderStage::Render`).
///
/// The [`Device`] is used to create gpu resources (buffers, bind groups, pipelines, etc.) and
/// the [`CommandEncoder`] is used to record a series of GPU operations.
pub struct RenderContext {
    pub device: Device,
    pub command_encoder: CommandEncoder,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[error("node does not exist")]
    InvalidNode(NodeLabel),
    #[error("output node slot does not exist")]
    InvalidOutputNodeSlot(SlotLabel),
    #[error("input node slot does not exist")]
    InvalidInputNodeSlot(SlotLabel),
    #[error("node does not match the given type")]
    WrongNodeType,
    #[error("attempted to connect a node output slot to an incompatible input node slot")]
    MismatchedNodeSlots {
        output_node: NodeId,
        output_slot: usize,
        input_node: NodeId,
        input_slot: usize,
    },
    #[error("attempted to add an edge that already exists")]
    EdgeAlreadyExists(Edge),
    #[error("attempted to remove an edge that does not exist")]
    EdgeDoesNotExist(Edge),
    #[error("node has an unconnected input slot")]
    UnconnectedNodeInputSlot { node: NodeId, input_slot: usize },
    #[error("node has an unconnected output slot")]
    UnconnectedNodeOutputSlot { node: NodeId, output_slot: usize },
    #[error("node input slot already occupied")]
    NodeInputSlotAlreadyOccupied {
        node: NodeId,
        input_slot: usize,
        occupied_by_node: NodeId,
    },
}

/// Updates the [`RenderGraph`] with all of its nodes and then runs it to render the entire frame.
pub fn render_system(world: &mut World) {
    world.resource_scope(|world, mut graph: Mut<RenderGraph>| {
        graph.update(world);
    });
    let graph = world.resource::<RenderGraph>();
    let device = world.resource::<Device>();
    let queue = world.resource::<Queue>();

    if let Err(e) = RenderGraphRunner::run(
        graph,
        device.clone(), // TODO: is this clone really necessary?
        queue,
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
