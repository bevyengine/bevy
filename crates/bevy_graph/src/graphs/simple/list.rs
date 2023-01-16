use slotmap::{HopSlotMap, Key, SecondaryMap};

use crate::{
    error::GraphError,
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
        Graph, SimpleGraph,
    },
};

/// Implementation of a `SimpleGraph` which uses `Vec<(NodeIdx, EdgeIdx)>` for adjacencies
///
/// `SimpleGraph`s can only hold one edge between two nodes and can't have edges between the same node
#[derive(Clone)]
pub struct SimpleListGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, Vec<(NodeIdx, EdgeIdx)>>,
}

impl<N, E, const DIRECTED: bool> Graph<N, E> for SimpleListGraph<N, E, DIRECTED> {
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
    fn is_undirected(&self) -> bool {
        !DIRECTED
    }

    #[inline]
    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    #[inline]
    fn new_node(&mut self, node: N) -> NodeIdx {
        let idx = self.nodes.insert(node);
        self.adjacencies.insert(idx, Vec::new());
        idx
    }

    #[inline]
    fn get_node(&self, idx: NodeIdx) -> Result<&N, GraphError> {
        if self.nodes.contains_key(idx) {
            unsafe { Ok(self.get_node_unchecked(idx)) }
        } else {
            Err(GraphError::NodeIdxDoesntExist(idx))
        }
    }

    #[inline]
    unsafe fn get_node_unchecked(&self, idx: NodeIdx) -> &N {
        self.nodes.get_unchecked(idx)
    }

    #[inline]
    fn get_node_mut(&mut self, idx: NodeIdx) -> Result<&mut N, GraphError> {
        if self.nodes.contains_key(idx) {
            unsafe { Ok(self.get_node_unchecked_mut(idx)) }
        } else {
            Err(GraphError::NodeIdxDoesntExist(idx))
        }
    }

    #[inline]
    unsafe fn get_node_unchecked_mut(&mut self, idx: NodeIdx) -> &mut N {
        self.nodes.get_unchecked_mut(idx)
    }

    #[inline]
    fn has_node(&self, node: NodeIdx) -> bool {
        self.nodes.contains_key(node)
    }

    #[inline]
    fn get_edge(&self, edge: EdgeIdx) -> Result<&E, GraphError> {
        match self.edges.get(edge) {
            Some(e) => Ok(&e.data),
            None => Err(GraphError::EdgeIdxDoesntExist(edge)),
        }
    }

    #[inline]
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> Result<&mut E, GraphError> {
        match self.edges.get_mut(edge) {
            Some(e) => Ok(&mut e.data),
            None => Err(GraphError::EdgeIdxDoesntExist(edge)),
        }
    }

    fn remove_edge(&mut self, edge: EdgeIdx) -> Result<E, GraphError> {
        if self.edges.contains_key(edge) {
            unsafe { Ok(self.remove_edge_unchecked(edge)) }
        } else {
            Err(GraphError::EdgeIdxDoesntExist(edge))
        }
    }

    #[inline]
    fn edges_between(&self, from: NodeIdx, to: NodeIdx) -> Result<Vec<EdgeIdx>, GraphError> {
        match self.edge_between(from, to) {
            Ok(Some(idx)) => Ok(vec![idx]),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    #[inline]
    unsafe fn edges_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> Vec<EdgeIdx> {
        vec![self.edge_between_unchecked(from, to)]
    }

    #[inline]
    fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)> {
        self.adjacencies.get(node).unwrap().to_vec()
    }

    fn remove_node(&mut self, node: NodeIdx) -> Result<N, GraphError> {
        if DIRECTED {
            let mut edges = vec![];
            for (edge, data) in &self.edges {
                let (src, dst) = data.indices();
                if dst == node || src == node {
                    edges.push(edge);
                }
            }
            for edge in edges {
                unsafe {
                    // SAFETY: we know it must exist
                    self.remove_edge_unchecked(edge); // TODO: can we have a `remove_edges` function?
                }
            }
            match self.nodes.remove(node) {
                Some(n) => {
                    unsafe {
                        // SAFETY: it will exist.
                        self.adjacencies.remove(node).unwrap_unchecked();
                    }
                    Ok(n)
                }
                None => Err(GraphError::NodeIdxDoesntExist(node)),
            }
        } else {
            for (_, edge) in self.edges_of(node) {
                unsafe {
                    // SAFETY: we know it must exist
                    self.remove_edge_unchecked(edge); // TODO: can we have a `remove_edges` function?
                }
            }
            match self.nodes.remove(node) {
                Some(n) => {
                    unsafe {
                        // SAFETY: it will exist.
                        self.adjacencies.remove(node).unwrap_unchecked();
                    }
                    Ok(n)
                }
                None => Err(GraphError::NodeIdxDoesntExist(node)),
            }
        }
    }

    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> Result<EdgeIdx, GraphError> {
        if from == to {
            Err(GraphError::EdgeBetweenSameNode(from))
        } else if DIRECTED {
            if let Some(from_edges) = self.adjacencies.get(from) {
                if from_edges.iter().any(|(n, _)| *n == to) {
                    Err(GraphError::EdgeBetweenAlreadyExists(from, to))
                } else if self.has_node(to) {
                    unsafe { Ok(self.new_edge_unchecked(from, to, edge)) }
                } else {
                    Err(GraphError::NodeIdxDoesntExist(to))
                }
            } else {
                Err(GraphError::NodeIdxDoesntExist(from))
            }
        } else if let Some(node_edges) = self.adjacencies.get(from) {
            if node_edges.iter().any(|(n, _)| *n == to) {
                Err(GraphError::EdgeBetweenAlreadyExists(from, to))
            } else if let Some(other_edges) = self.adjacencies.get(to) {
                if other_edges.iter().any(|(n, _)| *n == from) {
                    Err(GraphError::EdgeBetweenAlreadyExists(to, from))
                } else {
                    unsafe { Ok(self.new_edge_unchecked(from, to, edge)) }
                }
            } else {
                Err(GraphError::NodeIdxDoesntExist(to))
            }
        } else {
            Err(GraphError::NodeIdxDoesntExist(from))
        }
    }

    unsafe fn new_edge_unchecked(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(Edge {
            src: from,
            dst: to,
            data: edge,
        });
        if DIRECTED {
            self.adjacencies.get_unchecked_mut(from).push((to, idx));
        } else {
            self.adjacencies.get_unchecked_mut(from).push((to, idx));
            self.adjacencies.get_unchecked_mut(to).push((from, idx));
        }
        idx
    }

    unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E {
        if DIRECTED {
            let (from, to) = self.edges.get_unchecked(edge).indices();
            let list = self.adjacencies.get_unchecked_mut(from);
            list.swap_remove(find_edge(list, to).unwrap()); // TODO: remove or swap_remove ?
        } else {
            let (node, other) = self.edges.get_unchecked(edge).indices();
            let list = self.adjacencies.get_unchecked_mut(node);
            list.swap_remove(find_edge(list, other).unwrap()); // TODO: remove or swap_remove ?
            let list = self.adjacencies.get_unchecked_mut(other);
            list.swap_remove(find_edge(list, node).unwrap()); // TODO: remove or swap_remove ?
        }
        self.edges.remove(edge).unwrap().data
    }
}

impl<N, E, const DIRECTED: bool> SimpleGraph<N, E> for SimpleListGraph<N, E, DIRECTED> {
    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> Result<Option<EdgeIdx>, GraphError> {
        if self.adjacencies.contains_key(from) {
            unsafe {
                let idx = self.edge_between_unchecked(from, to);
                if idx.is_null() {
                    Ok(None)
                } else {
                    Ok(Some(idx))
                }
            }
        } else {
            Err(GraphError::NodeIdxDoesntExist(from))
        }
    }

    unsafe fn edge_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx {
        if let Some(idx) = self
            .adjacencies
            .get_unchecked(from)
            .iter()
            .find_map(|(other_node, idx)| if *other_node == to { Some(*idx) } else { None })
        // we know it simple graph can only have 1 edge so `find_map` is enough
        {
            idx
        } else {
            EdgeIdx::null()
        }
    }
}

// Util function
#[inline]
fn find_edge(list: &[(NodeIdx, EdgeIdx)], node: NodeIdx) -> Option<usize> {
    list.iter()
        .position(|(node_idx, _edge_idx)| *node_idx == node)
}

#[cfg(test)]
mod test {
    use crate::simple_graph_tests;

    simple_graph_tests!(super::SimpleListGraph);
}
