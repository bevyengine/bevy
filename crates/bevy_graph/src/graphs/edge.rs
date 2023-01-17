use super::keys::NodeIdx;

/// An edge between nodes that store data of type `E`.
#[derive(Clone)]
pub struct Edge<E>(pub NodeIdx, pub NodeIdx, pub E);
