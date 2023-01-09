mod context;
mod edge;
mod graph;
mod node;
mod node_slot;

pub use context::*;
pub use edge::*;
pub use graph::*;
pub use node::*;
pub use node_slot::*;

use crate::{render_resource::CommandEncoder, renderer::Device};
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
