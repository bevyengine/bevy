use thiserror::Error;

use crate::graphs::keys::{EdgeIdx, NodeIdx};

/// An error that can occur when traversing or manipulating a graph data structure
#[derive(Debug, Error)]
pub enum GraphError {
    /// the given `NodeIdx` is not preset in the graph
    #[error("node by given NodeIdx `{0:?}` doesn't exist in this graph")]
    NodeIdxDoesntExist(NodeIdx),
    /// the given `EdgeIdx` is not preset in the graph
    #[error("edge by given EdgeIdx `{0:?}` doesn't exist in this graph")]
    EdgeIdxDoesntExist(EdgeIdx),
    /// there is no edge between the two nodes in the graph
    #[error("edge between nodes `{0:?}` and `{1:?}` doesn't exist in this graph")]
    EdgeBetweenDoesntExist(NodeIdx, NodeIdx),
    /// there is already an edge between those two nodes
    #[error("edge between nodes `{0:?}` and `{1:?}` is already preset in this graph. would you like to use MultiGraph instead?")]
    EdgeBetweenAlreadyExists(NodeIdx, NodeIdx),
    /// `SimpleGraph`s can't hold an edge between the same node
    #[error("cannot create an edge between node `{0:?}` and itself in a SimpleGraph. use MultiGraph instead")]
    EdgeBetweenSameNode(NodeIdx),
}
