use thiserror::Error;

use crate::graphs::keys::{EdgeIdx, NodeIdx};

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("node by given NodeIdx `{0:?}` doesn't exist in this graph")]
    NodeIdxDoesntExist(NodeIdx),
    #[error("edge by given EdgeIdx `{0:?}` doesn't exist in this graph")]
    EdgeIdxDoesntExist(EdgeIdx),
    #[error("edge between nodes `{0:?}` and `{1:?}` doesn't exist in this graph")]
    EdgeBetweenDoesntExist(NodeIdx, NodeIdx),
    #[error("edge between nodes `{0:?}` and `{1:?}` is already preset in this graph")]
    EdgeBetweenAlreadyExists(NodeIdx, NodeIdx),
    #[error("cannot create an edge between node `{0:?}` and itself in a SimpleGraph. use MultiGraph instead")]
    EdgeBetweenSameNode(NodeIdx),
}

pub type GraphResult<T> = Result<T, GraphError>;
