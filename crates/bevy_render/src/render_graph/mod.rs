mod app;
mod context;
mod edge;
mod graph;
mod node;
mod node_slot;

pub use app::*;
pub use context::*;
pub use edge::*;
pub use graph::*;
pub use node::*;
pub use node_slot::*;

use derive_more::derive::{Display, Error};

#[derive(Error, Display, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[display("node {_0:?} does not exist")]
    #[error(ignore)]
    InvalidNode(InternedRenderLabel),
    #[display("output node slot does not exist")]
    #[error(ignore)]
    InvalidOutputNodeSlot(SlotLabel),
    #[display("input node slot does not exist")]
    #[error(ignore)]
    InvalidInputNodeSlot(SlotLabel),
    #[display("node does not match the given type")]
    WrongNodeType,
    #[display("attempted to connect output slot {output_slot} from node {output_node:?} to incompatible input slot {input_slot} from node {input_node:?}")]
    MismatchedNodeSlots {
        output_node: InternedRenderLabel,
        output_slot: usize,
        input_node: InternedRenderLabel,
        input_slot: usize,
    },
    #[display("attempted to add an edge that already exists")]
    #[error(ignore)]
    EdgeAlreadyExists(Edge),
    #[display("attempted to remove an edge that does not exist")]
    #[error(ignore)]
    EdgeDoesNotExist(Edge),
    #[display("node {node:?} has an unconnected input slot {input_slot}")]
    UnconnectedNodeInputSlot {
        node: InternedRenderLabel,
        input_slot: usize,
    },
    #[display("node {node:?} has an unconnected output slot {output_slot}")]
    UnconnectedNodeOutputSlot {
        node: InternedRenderLabel,
        output_slot: usize,
    },
    #[display("node {node:?} input slot {input_slot} already occupied by {occupied_by_node:?}")]
    NodeInputSlotAlreadyOccupied {
        node: InternedRenderLabel,
        input_slot: usize,
        occupied_by_node: InternedRenderLabel,
    },
}
