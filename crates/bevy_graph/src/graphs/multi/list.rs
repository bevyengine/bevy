use slotmap::{HopSlotMap, SecondaryMap};

use crate::{
    error::GraphError,
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
        Graph,
    },
    utils::{vecmap::VecMap, vecset::VecSet},
};

/// Implementation of a `MultiGraph` which uses `Vec<(NodeIdx, Vec<EdgeIdx>)>` for adjacencies
///
/// `MultiGraph`s can hold multiple edges between two nodes and edges between the same node
#[derive(Clone)]
pub struct MultiListGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, Vec<(NodeIdx, Vec<EdgeIdx>)>>,
}

impl<N, E, const DIRECTED: bool> Graph<N, E> for MultiListGraph<N, E, DIRECTED> {
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
        true
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
        self.adjacencies.insert(idx, Vec::new());
        idx
    }

    fn try_add_edge(
        &mut self,
        src: NodeIdx,
        dst: NodeIdx,
        value: E,
    ) -> Result<EdgeIdx, GraphError> {
        if !self.has_node(src) {
            Err(GraphError::NodeNotFound(src))
        } else if !self.has_node(dst) {
            Err(GraphError::NodeNotFound(dst))
        } else {
            unsafe {
                let idx = self.edges.insert(Edge(src, dst, value));
                self.adjacencies
                    .get_unchecked_mut(src)
                    .get_value_or_default_mut(dst)
                    .push(idx);
                if !DIRECTED {
                    self.adjacencies
                        .get_unchecked_mut(dst)
                        .get_value_or_default_mut(src)
                        .push(idx);
                }
                Ok(idx)
            }
        }
    }

    #[inline]
    fn has_node(&self, node: NodeIdx) -> bool {
        self.nodes.contains_key(node)
    }

    fn contains_edge_between(&self, src: NodeIdx, dst: NodeIdx) -> bool {
        self.adjacencies.get(src).unwrap().contains_key(dst)
    }

    fn remove_node(&mut self, index: NodeIdx) -> Option<N> {
        todo!()
    }

    fn remove_edge(&mut self, index: EdgeIdx) -> Option<E> {
        if let Some(Edge(src, dst, value)) = self.edges.remove(index) {
            unsafe {
                self.adjacencies
                    .get_unchecked_mut(src)
                    .get_value_mut(dst)
                    .unwrap()
                    .remove_by_value(&index);
                if !DIRECTED {
                    self.adjacencies
                        .get_unchecked_mut(dst)
                        .get_value_mut(src)
                        .unwrap()
                        .remove_by_value(&index);
                }
            }
            Some(value)
        } else {
            None
        }
    }

    fn clear_edges(&mut self) {
        self.adjacencies.values_mut().for_each(|list| list.clear());
        self.edges.clear();
    }

    fn clear(&mut self) {
        self.adjacencies.clear();
        self.edges.clear();
        self.nodes.clear();
    }

    #[inline]
    fn get_node(&self, index: NodeIdx) -> Option<&N> {
        self.nodes.get(index)
    }

    #[inline]
    fn get_node_mut(&mut self, index: NodeIdx) -> Option<&mut N> {
        self.nodes.get_mut(index)
    }

    #[inline]
    fn get_edge(&self, index: EdgeIdx) -> Option<&E> {
        if let Some(Edge(_, _, value)) = self.edges.get(index) {
            Some(value)
        } else {
            None
        }
    }

    #[inline]
    fn get_edge_mut(&mut self, index: EdgeIdx) -> Option<&mut E> {
        if let Some(Edge(_, _, value)) = self.edges.get_mut(index) {
            Some(value)
        } else {
            None
        }
    }
}
