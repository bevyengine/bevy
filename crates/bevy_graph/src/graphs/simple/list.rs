use slotmap::{HopSlotMap, Key, SecondaryMap};

use crate::{
    error::{GraphError, GraphResult},
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
        SimpleGraph,
    },
    impl_graph,
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

impl_graph! {
    impl COMMON for SimpleListGraph {
        fn new() -> Self {
            Self {
                nodes: HopSlotMap::with_key(),
                edges: HopSlotMap::with_key(),
                adjacencies: SecondaryMap::new(),
            }
        }

        #[inline]
        fn count(&self) -> usize {
            self.nodes.len()
        }

        #[inline]
        fn new_node(&mut self, node: N) -> NodeIdx {
            let idx = self.nodes.insert(node);
            self.adjacencies.insert(idx, Vec::new());
            idx
        }

        #[inline]
        fn get_node(&self, idx: NodeIdx) -> GraphResult<&N> {
            if self.nodes.contains_key(idx) {
                unsafe {
                    Ok(self.get_node_unchecked(idx))
                }
            } else {
                Err(GraphError::NodeIdxDoesntExist(idx))
            }
        }

        #[inline]
        unsafe fn get_node_unchecked(&self, idx: NodeIdx) -> &N {
            self.nodes.get_unchecked(idx)
        }

        #[inline]
        fn get_node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N> {
            if self.nodes.contains_key(idx) {
                unsafe {
                    Ok(self.get_node_unchecked_mut(idx))
                }
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
        fn get_edge(&self, edge: EdgeIdx) -> GraphResult<&E> {
            match self.edges.get(edge) {
                Some(e) => Ok(&e.data),
                None => Err(GraphError::EdgeIdxDoesntExist(edge))
            }
        }

        #[inline]
        fn get_edge_mut(&mut self, edge: EdgeIdx) -> GraphResult<&mut E> {
            match self.edges.get_mut(edge) {
                Some(e) => Ok(&mut e.data),
                None => Err(GraphError::EdgeIdxDoesntExist(edge))
            }
        }

        fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E> {
            if self.edges.contains_key(edge) {
                unsafe {
                    Ok(self.remove_edge_unchecked(edge))
                }
            } else {
                Err(GraphError::EdgeIdxDoesntExist(edge))
            }
        }

        #[inline]
        fn edges_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Vec<EdgeIdx>> {
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
    }

    impl COMMON?undirected {
        fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N> {
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
                },
                None => Err(GraphError::NodeIdxDoesntExist(node))
            }
        }

        fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> GraphResult<EdgeIdx> {
            if node == other {
                Err(GraphError::EdgeBetweenSameNode(node))
            } else if let Some(node_edges) = self.adjacencies.get(node) {
                if node_edges.iter().any(|(n, _)| *n == other) {
                    Err(GraphError::EdgeBetweenAlreadyExists(node, other))
                } else if let Some(other_edges) = self.adjacencies.get(other) {
                    if other_edges.iter().any(|(n, _)| *n == node) {
                        Err(GraphError::EdgeBetweenAlreadyExists(other, node))
                    } else {
                        unsafe {
                            Ok(self.new_edge_unchecked(node, other, edge))
                        }
                    }
                } else {
                    Err(GraphError::NodeIdxDoesntExist(other))
                }
            } else {
                Err(GraphError::NodeIdxDoesntExist(node))
            }
        }

        unsafe fn new_edge_unchecked(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx {
            let idx = self.edges.insert(Edge {
                src: node,
                dst: other,
                data: edge,
            });
            self.adjacencies.get_unchecked_mut(node).push((other, idx));
            self.adjacencies.get_unchecked_mut(other).push((node, idx));
            idx
        }

        unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E {
            let (node, other) = self.edges.get_unchecked(edge).indices();
            let list = self.adjacencies.get_unchecked_mut(node);
            list.swap_remove(find_edge(list, other).unwrap()); // TODO: remove or swap_remove ?
            let list = self.adjacencies.get_unchecked_mut(other);
            list.swap_remove(find_edge(list, node).unwrap()); // TODO: remove or swap_remove ?
            self.edges.remove(edge).unwrap().data
        }
    }

    impl COMMON?directed {
        fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N> {
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
                },
                None => Err(GraphError::NodeIdxDoesntExist(node))
            }
        }

        fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> GraphResult<EdgeIdx> {
            if from == to {
                Err(GraphError::EdgeBetweenSameNode(from))
            } else if let Some(from_edges) = self.adjacencies.get(from) {
                if from_edges.iter().any(|(n, _)| *n == to) {
                    Err(GraphError::EdgeBetweenAlreadyExists(from, to))
                } else if self.has_node(to) {
                    unsafe {
                        Ok(self.new_edge_unchecked(from, to, edge))
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
            self.adjacencies.get_unchecked_mut(from).push((to, idx));
            idx
        }

        unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E {
            let (from, to) = self.edges.get_unchecked(edge).indices();
            let list = self.adjacencies.get_unchecked_mut(from);
            list.swap_remove(find_edge(list, to).unwrap()); // TODO: remove or swap_remove ?
            self.edges.remove(edge).unwrap().data
        }
    }

    impl SIMPLE {
        fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Option<EdgeIdx>> {
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
            if let Some(idx) = self.adjacencies.get_unchecked(from).iter()
                .find_map(|(other_node, idx)| if *other_node == to { Some(*idx) } else { None }) // we know it simple graph can only have 1 edge so `find_map` is enough
            {
                idx
            } else {
                EdgeIdx::null()
            }
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
