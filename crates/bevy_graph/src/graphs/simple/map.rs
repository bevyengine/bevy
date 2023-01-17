use hashbrown::HashMap;
use slotmap::{HopSlotMap, SecondaryMap};

use crate::{
    error::GraphError,
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
        Graph,
    },
};

/// Implementation of a `SimpleGraph` which uses `HashMap<NodeIdx, EdgeIdx>` for adjacencies
///
/// `SimpleGraph`s can only hold one edge between two nodes and can't have edges between the same node
#[derive(Clone)]
pub struct SimpleMapGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, HashMap<NodeIdx, EdgeIdx>>,
}

impl<N, E, const DIRECTED: bool> Graph<N, E> for SimpleMapGraph<N, E, DIRECTED> {
    fn new() -> Self {
        Self {
            nodes: HopSlotMap::with_key(),
            edges: HopSlotMap::with_key(),
            adjacencies: SecondaryMap::new(),
        }
    }

    #[inline]
    fn is_directed(&self) -> bool {
        DIRECTED
    }

    #[inline]
    fn is_multigraph(&self) -> bool {
        false
    }

    #[inline]
    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn add_node(&mut self, node: N) -> NodeIdx {
        let idx = self.nodes.insert(node);
        self.adjacencies.insert(idx, HashMap::new());
        idx
    }

    fn add_edge(&mut self, src: NodeIdx, dst: NodeIdx, value: E) -> Result<EdgeIdx, GraphError> {
        if !self.has_node(src) {
            Err(GraphError::NodeNotFound(src))
        } else if !self.has_node(dst) {
            Err(GraphError::NodeNotFound(dst))
        } else if self.contains_edge_between(src, dst) {
            Err(GraphError::ContainsEdgeBetween)
        } else if src == dst {
            Err(GraphError::Loop)
        } else {
            unsafe { Ok(self.add_edge_unchecked(src, dst, value)) }
        }
    }

    unsafe fn add_edge_unchecked(&mut self, src: NodeIdx, dst: NodeIdx, value: E) -> EdgeIdx {
        let idx = self.edges.insert(Edge(src, dst, value));
        self.adjacencies.get_unchecked_mut(src).insert(dst, idx);
        if !DIRECTED {
            self.adjacencies.get_unchecked_mut(dst).insert(src, idx);
        }
        idx
    }

    #[inline]
    fn has_node(&self, node: NodeIdx) -> bool {
        self.nodes.contains_key(node)
    }

    fn contains_edge_between(&self, src: NodeIdx, dst: NodeIdx) -> bool {
        self.adjacencies[src].contains_key(&dst)
    }

    fn remove_node(&mut self, index: NodeIdx) -> Option<N> {
        todo!()
    }

    fn remove_edge(&mut self, index: EdgeIdx) -> Option<E> {
        if let Some(Edge(src, dst, value)) = self.edges.remove(index) {
            unsafe {
                self.adjacencies.get_unchecked_mut(src).remove(&dst);
                if !DIRECTED {
                    self.adjacencies.get_unchecked_mut(dst).remove(&src);
                }
            }
            Some(value)
        } else {
            None
        }
    }
}
