use crate::NodeIdx;

pub mod simple;

#[derive(Clone)]
pub struct Edge<E> {
    pub src: NodeIdx,
    pub dst: NodeIdx,
    pub data: E,
}
