use hashbrown::HashMap;
use slotmap::{HopSlotMap, SecondaryMap};

use crate::{
    error::GraphError,
    graphs::{
        adjacency_storage::AdjacencyStorage,
        edge::{Edge, EdgeMut, EdgeRef},
        keys::{EdgeIdx, NodeIdx},
        Graph,
    },
    iters,
    utils::vecset::VecSet,
};

/// Implementation of a `MultiGraph` which uses `HashMap<NodeIdx, Vec<EdgeIdx>>` for adjacencies
///
/// `MultiGraph`s can hold multiple edges between two nodes and edges between the same node
#[derive(Clone)]
pub struct MultiMapGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, AdjacencyStorage<HashMap<NodeIdx, Vec<EdgeIdx>>>>,
}

impl<N, E, const DIRECTED: bool> Graph<N, E> for MultiMapGraph<N, E, DIRECTED> {
    fn new() -> Self {
        Self {
            nodes: HopSlotMap::with_key(),
            edges: HopSlotMap::with_key(),
            adjacencies: SecondaryMap::new(),
        }
    }

    fn with_capacity(node_capacity: usize, edge_capacity: usize) -> Self {
        Self {
            nodes: HopSlotMap::with_capacity_and_key(node_capacity),
            edges: HopSlotMap::with_capacity_and_key(edge_capacity),
            adjacencies: SecondaryMap::new(),
        }
    }

    #[inline]
    fn capacity(&self) -> (usize, usize) {
        (self.nodes.capacity(), self.edges.capacity())
    }

    #[inline]
    fn node_capacity(&self) -> usize {
        self.nodes.capacity()
    }

    #[inline]
    fn edge_capacity(&self) -> usize {
        self.edges.capacity()
    }

    #[inline]
    fn reserve_nodes(&mut self, additional: usize) {
        self.nodes.reserve(additional);
    }

    #[inline]
    fn reserve_edges(&mut self, additional: usize) {
        self.edges.reserve(additional);
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
        let storage = if DIRECTED {
            AdjacencyStorage::Directed(HashMap::new(), HashMap::new())
        } else {
            AdjacencyStorage::Undirected(HashMap::new())
        };
        self.adjacencies.insert(idx, storage);
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
                    .outgoing_mut()
                    .entry(dst)
                    .or_default()
                    .push(idx);
                self.adjacencies
                    .get_unchecked_mut(dst)
                    .incoming_mut()
                    .entry(src)
                    .or_default()
                    .push(idx);
                Ok(idx)
            }
        }
    }

    #[inline]
    fn has_node(&self, node: NodeIdx) -> bool {
        self.nodes.contains_key(node)
    }

    fn contains_edge_between(&self, src: NodeIdx, dst: NodeIdx) -> bool {
        self.adjacencies
            .get(src)
            .unwrap()
            .outgoing()
            .contains_key(&dst)
    }

    fn remove_node(&mut self, _index: NodeIdx) -> Option<N> {
        todo!()
    }

    fn remove_edge(&mut self, index: EdgeIdx) -> Option<E> {
        if let Some(Edge(src, dst, value)) = self.edges.remove(index) {
            unsafe {
                self.adjacencies
                    .get_unchecked_mut(src)
                    .outgoing_mut()
                    .get_mut(&dst)
                    .unwrap()
                    .remove_by_value(&index);
                self.adjacencies
                    .get_unchecked_mut(dst)
                    .incoming_mut()
                    .get_mut(&src)
                    .unwrap()
                    .remove_by_value(&index);
            }
            Some(value)
        } else {
            None
        }
    }

    fn clear_edges(&mut self) {
        self.adjacencies
            .values_mut()
            .for_each(|map| map.for_each_mut(HashMap::clear));
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
    unsafe fn get_edge_raw(&mut self, index: EdgeIdx) -> Option<&mut Edge<E>> {
        self.edges.get_mut(index)
    }

    #[inline]
    fn get_edge(&self, index: EdgeIdx) -> Option<EdgeRef<E>> {
        self.edges.get(index).map(|edge| edge.as_ref_edge())
    }

    #[inline]
    fn get_edge_mut(&mut self, index: EdgeIdx) -> Option<EdgeMut<E>> {
        self.edges.get_mut(index).map(|edge| edge.as_mut_edge())
    }

    fn degree(&self, _index: NodeIdx) -> usize {
        todo!()
    }

    type NodeIndices<'n> = slotmap::hop::Keys<'n, NodeIdx, N> where Self: 'n;
    fn node_indices(&self) -> Self::NodeIndices<'_> {
        self.nodes.keys()
    }

    type Nodes<'n> = slotmap::hop::Values<'n, NodeIdx, N> where Self: 'n;
    fn nodes(&self) -> Self::Nodes<'_> {
        self.nodes.values()
    }

    type NodesMut<'n> = slotmap::hop::ValuesMut<'n, NodeIdx, N> where Self: 'n;
    fn nodes_mut(&mut self) -> Self::NodesMut<'_> {
        self.nodes.values_mut()
    }

    type EdgeIndices<'e> = slotmap::hop::Keys<'e, EdgeIdx, Edge<E>> where Self: 'e;
    fn edge_indices(&self) -> Self::EdgeIndices<'_> {
        self.edges.keys()
    }

    type Edges<'e> = iters::EdgesRef<'e, E, slotmap::hop::Values<'e, EdgeIdx, Edge<E>>> where Self: 'e;
    fn edges(&self) -> Self::Edges<'_> {
        iters::EdgesRef::new(self.edges.values())
    }

    type EdgesMut<'e> = iters::EdgesMut<'e, E, slotmap::hop::ValuesMut<'e, EdgeIdx, Edge<E>>> where Self: 'e;
    fn edges_mut(&mut self) -> Self::EdgesMut<'_> {
        iters::EdgesMut::new(self.edges.values_mut())
    }

    type IncomingEdgesOf<'e> = iters::EdgesByIdx<'e, N, E, Self, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn incoming_edges_of(&self, index: NodeIdx) -> Self::IncomingEdgesOf<'_> {
        iters::EdgesByIdx::new(self.adjacencies[index].incoming().values().flatten(), self)
    }

    type IncomingEdgesOfMut<'e> = iters::EdgesByIdxMut<'e, N, E, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn incoming_edges_of_mut(&mut self, index: NodeIdx) -> Self::IncomingEdgesOfMut<'_> {
        iters::EdgesByIdxMut::new(
            self.adjacencies[index].incoming().values().flatten(),
            &mut self.edges,
        )
    }

    type OutgoingEdgesOf<'e> = iters::EdgesByIdx<'e, N, E, Self, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn outgoing_edges_of(&self, index: NodeIdx) -> Self::IncomingEdgesOf<'_> {
        iters::EdgesByIdx::new(self.adjacencies[index].outgoing().values().flatten(), self)
    }

    type OutgoingEdgesOfMut<'e> = iters::EdgesByIdxMut<'e, N, E, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn outgoing_edges_of_mut(&mut self, index: NodeIdx) -> Self::IncomingEdgesOfMut<'_> {
        iters::EdgesByIdxMut::new(
            self.adjacencies[index].outgoing().values().flatten(),
            &mut self.edges,
        )
    }

    #[inline]
    fn in_degree(&self, index: NodeIdx) -> usize {
        self.adjacencies[index].incoming().len()
    }

    #[inline]
    fn out_degree(&self, index: NodeIdx) -> usize {
        self.adjacencies[index].outgoing().len()
    }
}
