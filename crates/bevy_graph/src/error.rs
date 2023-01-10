use crate::NodeIdx;

pub enum GraphError {
    NodeDoesntExist(NodeIdx),
}

pub type GraphResult<T> = Result<T, GraphError>;
