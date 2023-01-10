use crate::graphs::keys::{EdgeIdx, NodeIdx};

#[derive(Debug)]
pub enum GraphError {
    NodeDoesntExist(NodeIdx),
    EdgeDoesntExist(EdgeIdx),
}

pub type GraphResult<T> = Result<T, GraphError>;
