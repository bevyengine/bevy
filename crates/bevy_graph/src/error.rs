use crate::graphs::keys::{EdgeIdx, NodeIdx};

#[derive(Debug)]
/// An error that can occur when traversing or manipulating a graph data structure
pub enum GraphError {
    NodeIdxDoesntExist(NodeIdx),
    EdgeIdxDoesntExist(EdgeIdx),
    EdgeBetweenDoesntExist(NodeIdx, NodeIdx),
    EdgeBetweenAlreadyExists(NodeIdx, NodeIdx),
    EdgeBetweenSameNode(NodeIdx),
}

pub type GraphResult<T> = Result<T, GraphError>;
