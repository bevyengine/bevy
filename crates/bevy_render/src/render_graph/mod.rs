mod command;
mod edge;
mod graph;
mod node;
mod node_slot;
pub mod nodes;
mod schedule;
pub mod system;
pub use command::*;
pub use edge::*;
pub use graph::*;
pub use node::*;
pub use node_slot::*;
pub use schedule::*;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[error("Node does not exist")]
    InvalidNode(NodeLabel),
    #[error("Node slot does not exist")]
    InvalidNodeSlot(SlotLabel),
    #[error("Node does not match the given type")]
    WrongNodeType,
    #[error("Attempted to connect a node output slot to an incompatible input node slot")]
    MismatchedNodeSlots {
        output_node: NodeId,
        output_slot: usize,
        input_node: NodeId,
        input_slot: usize,
    },
    #[error("Attempted to add an edge that already exists")]
    EdgeAlreadyExists(Edge),
    #[error("Node has an unconnected input slot.")]
    UnconnectedNodeInputSlot { node: NodeId, input_slot: usize },
    #[error("Node has an unconnected output slot.")]
    UnconnectedNodeOutputSlot { node: NodeId, output_slot: usize },
    #[error("Node input slot already occupied")]
    NodeInputSlotAlreadyOccupied {
        node: NodeId,
        input_slot: usize,
        occupied_by_node: NodeId,
    },
}
