use hashbrown::HashMap;
use slotmap::{HopSlotMap, SecondaryMap};

use crate::{
    error::{GraphError, GraphResult},
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
        SimpleGraph,
    },
    impl_graph,
};

#[derive(Clone)]
pub struct SimpleMapGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, HashMap<NodeIdx, EdgeIdx>>,
}

impl<N, E, const DIRECTED: bool> SimpleMapGraph<N, E, DIRECTED> {}

impl_graph! {
    impl COMMON for SimpleMapGraph {
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
            self.adjacencies.insert(idx, HashMap::new());
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
                Err(e) => Err(e)
            }
        }

        #[inline]
        unsafe fn edges_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> Vec<EdgeIdx> {
            vec![self.edge_between_unchecked(from, to)]
        }

        #[inline]
        fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)> {
            self.adjacencies
                .get(node)
                .unwrap()
                .iter()
                .map(|(node, edge)| (*node, *edge))
                .collect()
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
                if node_edges.contains_key(&other) {
                    Err(GraphError::EdgeBetweenAlreadyExists(node, other))
                } else if let Some(other_edges) = self.adjacencies.get(other) {
                    if other_edges.contains_key(&node) {
                        Err(GraphError::EdgeBetweenAlreadyExists(node, other))
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
            self.adjacencies.get_unchecked_mut(node).insert_unique_unchecked(other, idx);
            self.adjacencies.get_unchecked_mut(other).insert_unique_unchecked(node, idx);
            idx
        }

        unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E {
            let (node, other) = self.edges.get_unchecked(edge).indices();
            self.adjacencies.get_unchecked_mut(node).remove(&other);
            self.adjacencies.get_unchecked_mut(other).remove(&node);
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
                if from_edges.contains_key(&to) {
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
            self.adjacencies.get_unchecked_mut(from).insert_unique_unchecked(to, idx);
            idx
        }

        unsafe fn remove_edge_unchecked(&mut self, edge: EdgeIdx) -> E {
            let (from, to) = self.edges.get_unchecked(edge).indices();
            self.adjacencies.get_unchecked_mut(from).remove(&to);
            self.edges.remove(edge).unwrap().data
        }
    }

    impl SIMPLE {
        fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> GraphResult<Option<EdgeIdx>> {
            if let Some(from_edges) = self.adjacencies.get(from) {
                if from_edges.contains_key(&to) {
                    unsafe {
                        Ok(Some(self.edge_between_unchecked(from, to)))
                    }
                } else {
                    Ok(None)
                }
            } else {
                Err(GraphError::NodeIdxDoesntExist(from))
            }
        }

        unsafe fn edge_between_unchecked(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx {
            self.adjacencies.get_unchecked(from).get(&to).cloned().unwrap()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::simple_graph_tests;

    simple_graph_tests!(super::SimpleMapGraph);
}
