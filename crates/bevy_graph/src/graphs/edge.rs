use super::keys::NodeIdx;

/// An edge between nodes that store data of type `E`.
#[derive(Clone)]
pub struct Edge<E> {
    /// the `NodeIdx` of the source node
    pub src: NodeIdx,
    /// the `NodeIdx` of the destination node
    pub dst: NodeIdx,
    /// the edge data
    pub data: E,
}

impl<E> Edge<E> {
    /// Returns the `src` and `dst` of the edge as a tuple
    #[inline]
    pub const fn indices(&self) -> (NodeIdx, NodeIdx) {
        (self.src, self.dst)
    }
}
