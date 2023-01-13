use super::keys::NodeIdx;

#[derive(Clone)]
pub struct Edge<E> {
    pub src: NodeIdx,
    pub dst: NodeIdx,
    pub data: E,
}

impl<E> Edge<E> {
    #[inline]
    pub const fn indices(&self) -> (NodeIdx, NodeIdx) {
        (self.src, self.dst)
    }
}
