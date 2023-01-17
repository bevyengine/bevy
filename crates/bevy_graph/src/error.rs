use thiserror::Error;

use crate::graphs::keys::NodeIdx;

/// An error that can occur when traversing or manipulating a graph data structure
#[derive(Debug, Error)]
pub enum GraphError {
    /// the given `NodeIdx` is not preset in the graph
    #[error("the given `{0:?}` isn't preset in the graph")]
    NodeNotFound(NodeIdx),
    /// there is already an edge between those nodes (not allowed in `SimpleGraph`)
    #[error("there is already an edge between those nodes (not allowed in `SimpleGraph`)")]
    Loop,
    /// the `src` and `dst` nodes are equal, the edge would be a loop (not allowed in `SimpleGraph`)
    #[error("the `src` and `dst` nodes are equal, the edge would be a loop (not allowed in `SimpleGraph`)")]
    ContainsEdgeBetween,
}
