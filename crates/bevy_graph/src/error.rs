use crate::graphs::keys::{EdgeIdx, NodeIdx};

#[derive(Debug)]
pub enum GraphError {
    NodeDoesntExist(NodeIdx),
    EdgeDoesntExist(EdgeIdx),
    EdgeAlreadyExists(NodeIdx, NodeIdx),
}

pub type GraphResult<T> = Result<T, GraphError>;
