use super::keys::NodeIdx;

#[derive(Clone)]
pub struct Edge<E> {
    pub src: NodeIdx,
    pub dst: NodeIdx,
    pub data: E,
}
