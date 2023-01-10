use crate::graphs::keys::NodeIdx;

#[derive(Debug)]
pub enum GraphError {
    NodeDoesntExist(NodeIdx),
}

pub type GraphResult<T> = Result<T, GraphError>;
