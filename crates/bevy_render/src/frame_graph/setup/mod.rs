pub mod app;
pub mod context;
pub mod edge;
pub mod graph;
pub mod graph_runner;
pub mod node;
pub mod node_slot;

pub use context::*;
pub use edge::*;
pub use graph::*;
pub use graph_runner::*;
pub use node::*;
pub use node_slot::*;

use thiserror::Error;

use crate::render_graph::InternedRenderLabel;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SetupGraphError {
    #[error("node {0:?} does not exist")]
    InvalidNode(InternedRenderLabel),
    #[error("output node slot does not exist")]
    InvalidOutputNodeSlot(SlotLabel),
    #[error("input node slot does not exist")]
    InvalidInputNodeSlot(SlotLabel),
    #[error("node does not match the given type")]
    WrongNodeType,
    #[error("attempted to connect output slot {output_slot} from node {output_node:?} to incompatible input slot {input_slot} from node {input_node:?}")]
    MismatchedNodeSlots {
        output_node: InternedRenderLabel,
        output_slot: usize,
        input_node: InternedRenderLabel,
        input_slot: usize,
    },
    #[error("attempted to add an edge that already exists")]
    EdgeAlreadyExists(Edge),
    #[error("attempted to remove an edge that does not exist")]
    EdgeDoesNotExist(Edge),
    #[error("node {node:?} has an unconnected input slot {input_slot}")]
    UnconnectedNodeInputSlot {
        node: InternedRenderLabel,
        input_slot: usize,
    },
    #[error("node {node:?} has an unconnected output slot {output_slot}")]
    UnconnectedNodeOutputSlot {
        node: InternedRenderLabel,
        output_slot: usize,
    },
    #[error("node {node:?} input slot {input_slot} already occupied by {occupied_by_node:?}")]
    NodeInputSlotAlreadyOccupied {
        node: InternedRenderLabel,
        input_slot: usize,
        occupied_by_node: InternedRenderLabel,
    },
}
