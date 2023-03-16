mod context;
mod edge;
mod graph;
mod node;

pub use context::*;
pub use edge::*;
pub use graph::*;
pub use node::*;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[error("node does not exist")]
    InvalidNode(NodeLabel),
    #[error("node does not match the given type")]
    WrongNodeType,
    #[error("attempted to add an edge that already exists")]
    EdgeAlreadyExists(Edge),
    #[error("attempted to remove an edge that does not exist")]
    EdgeDoesNotExist(Edge),
}
