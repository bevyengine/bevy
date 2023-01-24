use hashbrown::HashMap;
use slotmap::{HopSlotMap, SecondaryMap};

use crate::{
    algos::dfs::DepthFirstSearch,
    error::GraphError,
    graphs::{
        adjacency_storage::AdjacencyStorage,
        edge::{Edge, EdgeMut, EdgeRef},
        keys::{EdgeIdx, NodeIdx},
        DirectedGraph, Graph,
    },
    iters,
    utils::{iter_choice::IterChoice, vecset::VecSet, wrapped_iterator::WrappedIterator},
};

type MultiMapStorage = HashMap<NodeIdx, Vec<EdgeIdx>>;

/// Implementation of a `MultiGraph` which uses `HashMap<NodeIdx, Vec<EdgeIdx>>` for adjacencies
///
/// `MultiGraph`s can hold multiple edges between two nodes and edges between the same node
#[derive(Clone)]
pub struct MultiMapGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, AdjacencyStorage<MultiMapStorage>>,
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
            adjacencies: SecondaryMap::with_capacity(node_capacity),
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

    fn remove_node(&mut self, index: NodeIdx) -> Option<N> {
        if self.has_node(index) {
            let edges_to_remove = self
                .edges_of(index)
                .into_inner()
                .cloned()
                .collect::<Vec<EdgeIdx>>();
            for edge_idx in edges_to_remove {
                unsafe {
                    let Edge(src, dst, _) = self.edges.remove(edge_idx).unwrap_unchecked();
                    self.adjacencies
                        .get_unchecked_mut(src)
                        .outgoing_mut()
                        .get_mut(&dst)
                        .unwrap()
                        .remove_by_value(&edge_idx);
                    self.adjacencies
                        .get_unchecked_mut(dst)
                        .incoming_mut()
                        .get_mut(&src)
                        .unwrap()
                        .remove_by_value(&edge_idx);
                }
            }
            unsafe {
                self.adjacencies.remove(index).unwrap_unchecked();
            }
            self.nodes.remove(index)
        } else {
            None
        }
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

    fn degree(&self, index: NodeIdx) -> usize {
        if DIRECTED {
            self.in_degree(index) + self.out_degree(index)
        } else {
            self.adjacencies[index].incoming().len()
        }
    }

    type NodeIndices<'n> = slotmap::hop::Keys<'n, NodeIdx, N> where Self: 'n;
    fn node_indices(&self) -> Self::NodeIndices<'_> {
        self.nodes.keys()
    }

    unsafe fn nodes_raw(&self) -> &slotmap::HopSlotMap<NodeIdx, N> {
        &self.nodes
    }

    type Nodes<'n> = slotmap::hop::Values<'n, NodeIdx, N> where Self: 'n;
    fn nodes(&self) -> Self::Nodes<'_> {
        self.nodes.values()
    }

    unsafe fn nodes_mut_raw(&mut self) -> &mut HopSlotMap<NodeIdx, N> {
        &mut self.nodes
    }

    type NodesMut<'n> = slotmap::hop::ValuesMut<'n, NodeIdx, N> where Self: 'n;
    fn nodes_mut(&mut self) -> Self::NodesMut<'_> {
        self.nodes.values_mut()
    }

    type EdgeIndices<'e> = slotmap::hop::Keys<'e, EdgeIdx, Edge<E>> where Self: 'e;
    fn edge_indices(&self) -> Self::EdgeIndices<'_> {
        self.edges.keys()
    }

    unsafe fn edges_raw(&self) -> &HopSlotMap<EdgeIdx, Edge<E>> {
        &self.edges
    }

    type Edges<'e> = iters::EdgesRef<'e, E, slotmap::hop::Values<'e, EdgeIdx, Edge<E>>> where Self: 'e;
    fn edges(&self) -> Self::Edges<'_> {
        iters::EdgesRef::new(self.edges.values())
    }

    unsafe fn edges_mut_raw(&mut self) -> &mut HopSlotMap<EdgeIdx, Edge<E>> {
        &mut self.edges
    }

    type EdgesMut<'e> = iters::EdgesMut<'e, E, slotmap::hop::ValuesMut<'e, EdgeIdx, Edge<E>>> where Self: 'e;
    fn edges_mut(&mut self) -> Self::EdgesMut<'_> {
        iters::EdgesMut::new(self.edges.values_mut())
    }

    type EdgesOf<'e> = iters::EdgesByIdx<'e, E, &'e EdgeIdx, iters::LoopSafetyIter<'e, E, &'e EdgeIdx, IterChoice<&'e EdgeIdx, std::iter::Flatten<std::iter::Chain<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>, hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>>, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>>>> where Self: 'e;
    fn edges_of(&self, index: NodeIdx) -> Self::EdgesOf<'_> {
        let inner = if DIRECTED {
            IterChoice::new_first(
                self.adjacencies[index]
                    .incoming()
                    .values()
                    .chain(self.adjacencies[index].outgoing().values())
                    .flatten(),
            )
        } else {
            IterChoice::new_second(self.adjacencies[index].incoming().values().flatten())
        };
        iters::EdgesByIdx::new(iters::LoopSafetyIter::new(inner, &self.edges), &self.edges)
    }

    type EdgesOfMut<'e> = iters::EdgesByIdxMut<'e, E, &'e EdgeIdx, iters::LoopSafetyIter<'e, E, &'e EdgeIdx, IterChoice<&'e EdgeIdx, std::iter::Flatten<std::iter::Chain<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>, hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>>, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>>>> where Self: 'e;
    fn edges_of_mut(&mut self, index: NodeIdx) -> Self::EdgesOfMut<'_> {
        let inner = if DIRECTED {
            IterChoice::new_first(
                self.adjacencies[index]
                    .incoming()
                    .values()
                    .chain(self.adjacencies[index].outgoing().values())
                    .flatten(),
            )
        } else {
            IterChoice::new_second(self.adjacencies[index].incoming().values().flatten())
        };
        unsafe {
            // SAFETY: EdgesByIdxMut should'nt be able to do any bad stuff for LoopSafetyIter
            let ptr: *mut HopSlotMap<EdgeIdx, Edge<E>> = &mut self.edges;
            iters::EdgesByIdxMut::new(iters::LoopSafetyIter::new(inner, &*ptr), &mut self.edges)
        }
    }

    #[inline]
    fn in_degree(&self, index: NodeIdx) -> usize {
        self.adjacencies[index].incoming().len()
    }

    #[inline]
    fn out_degree(&self, index: NodeIdx) -> usize {
        self.adjacencies[index].outgoing().len()
    }

    type IncomingEdgesOf<'e> = iters::EdgesByIdx<'e, E, &'e EdgeIdx, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn incoming_edges_of(&self, index: NodeIdx) -> Self::IncomingEdgesOf<'_> {
        iters::EdgesByIdx::new(
            self.adjacencies[index].incoming().values().flatten(),
            &self.edges,
        )
    }

    type IncomingEdgesOfMut<'e> = iters::EdgesByIdxMut<'e, E, &'e EdgeIdx, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn incoming_edges_of_mut(&mut self, index: NodeIdx) -> Self::IncomingEdgesOfMut<'_> {
        iters::EdgesByIdxMut::new(
            self.adjacencies[index].incoming().values().flatten(),
            &mut self.edges,
        )
    }

    type OutgoingEdgesOf<'e> = iters::EdgesByIdx<'e, E, &'e EdgeIdx, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn outgoing_edges_of(&self, index: NodeIdx) -> Self::IncomingEdgesOf<'_> {
        iters::EdgesByIdx::new(
            self.adjacencies[index].outgoing().values().flatten(),
            &self.edges,
        )
    }

    type OutgoingEdgesOfMut<'e> = iters::EdgesByIdxMut<'e, E, &'e EdgeIdx, std::iter::Flatten<hashbrown::hash_map::Values<'e, NodeIdx, Vec<EdgeIdx>>>> where Self: 'e;
    fn outgoing_edges_of_mut(&mut self, index: NodeIdx) -> Self::IncomingEdgesOfMut<'_> {
        iters::EdgesByIdxMut::new(
            self.adjacencies[index].outgoing().values().flatten(),
            &mut self.edges,
        )
    }

    type InNeighbors<'n> = iters::NodesByIdx<'n, N, &'n NodeIdx, hashbrown::hash_map::Keys<'n, NodeIdx, Vec<EdgeIdx>>> where Self: 'n;
    fn in_neighbors(&self, index: NodeIdx) -> Self::InNeighbors<'_> {
        iters::NodesByIdx::new(self.adjacencies[index].incoming().keys(), &self.nodes)
    }

    type InNeighborsMut<'n> = iters::NodesByIdxMut<'n, N, &'n NodeIdx, hashbrown::hash_map::Keys<'n, NodeIdx, Vec<EdgeIdx>>> where Self: 'n;
    fn in_neighbors_mut(&mut self, index: NodeIdx) -> Self::InNeighborsMut<'_> {
        iters::NodesByIdxMut::new(self.adjacencies[index].incoming().keys(), &mut self.nodes)
    }

    type OutNeighbors<'n> = iters::NodesByIdx<'n, N, &'n NodeIdx, hashbrown::hash_map::Keys<'n, NodeIdx, Vec<EdgeIdx>>> where Self: 'n;
    fn out_neighbors(&self, index: NodeIdx) -> Self::OutNeighbors<'_> {
        iters::NodesByIdx::new(self.adjacencies[index].outgoing().keys(), &self.nodes)
    }

    type OutNeighborsMut<'n> = iters::NodesByIdxMut<'n, N, &'n NodeIdx, hashbrown::hash_map::Keys<'n, NodeIdx, Vec<EdgeIdx>>> where Self: 'n;
    fn out_neighbors_mut(&mut self, index: NodeIdx) -> Self::OutNeighborsMut<'_> {
        iters::NodesByIdxMut::new(
            self.adjacencies[index].outgoing_mut().keys(),
            &mut self.nodes,
        )
    }

    type Sources<'n> = iters::SourcesSinks<&'n N, iters::ZipInDegree<'n, MultiMapStorage, &'n N, slotmap::hop::Iter<'n, NodeIdx, N>>> where Self: 'n;
    fn sources(&self) -> Self::Sources<'_> {
        iters::SourcesSinks::new(iters::ZipInDegree::new(
            self.nodes.iter(),
            &self.adjacencies,
        ))
    }

    type SourcesMut<'n> = iters::SourcesSinks<&'n mut N, iters::ZipInDegree<'n, MultiMapStorage, &'n mut N, slotmap::hop::IterMut<'n, NodeIdx, N>>> where Self: 'n;
    fn sources_mut(&mut self) -> Self::SourcesMut<'_> {
        iters::SourcesSinks::new(iters::ZipInDegree::new(
            self.nodes.iter_mut(),
            &self.adjacencies,
        ))
    }

    type Sinks<'n> = iters::SourcesSinks<&'n N, iters::ZipOutDegree<'n, MultiMapStorage, &'n N, slotmap::hop::Iter<'n, NodeIdx, N>>> where Self: 'n;
    fn sinks(&self) -> Self::Sinks<'_> {
        iters::SourcesSinks::new(iters::ZipOutDegree::new(
            self.nodes.iter(),
            &self.adjacencies,
        ))
    }

    type SinksMut<'n> = iters::SourcesSinks<&'n mut N, iters::ZipOutDegree<'n, MultiMapStorage, &'n mut N, slotmap::hop::IterMut<'n, NodeIdx, N>>> where Self: 'n;
    fn sinks_mut(&mut self) -> Self::SinksMut<'_> {
        iters::SourcesSinks::new(iters::ZipOutDegree::new(
            self.nodes.iter_mut(),
            &self.adjacencies,
        ))
    }
}

impl<N, E> DirectedGraph<N, E> for MultiMapGraph<N, E, true> {
    fn reverse(&mut self) {
        self.adjacencies
            .values_mut()
            .for_each(|map| map.for_each_mut(HashMap::clear));

        for (index, Edge(src, dst, _)) in &mut self.edges {
            std::mem::swap(src, dst);
            self.adjacencies[*dst]
                .outgoing_mut()
                .get_mut(src)
                .unwrap()
                .push(index);
            self.adjacencies[*src]
                .incoming_mut()
                .get_mut(dst)
                .unwrap()
                .push(index);
        }
    }

    fn reverse_edge(&mut self, index: EdgeIdx) {
        if let Some(Edge(src, dst, _)) = self.edges.get_mut(index) {
            self.adjacencies[*src]
                .outgoing_mut()
                .get_mut(dst)
                .unwrap()
                .remove_by_value(&index);
            self.adjacencies[*dst]
                .incoming_mut()
                .get_mut(src)
                .unwrap()
                .remove_by_value(&index);
            std::mem::swap(src, dst);
            self.adjacencies[*dst]
                .outgoing_mut()
                .get_mut(src)
                .unwrap()
                .push(index);
            self.adjacencies[*src]
                .incoming_mut()
                .get_mut(dst)
                .unwrap()
                .push(index);
        }
    }

    type Ancestors<'n> = iters::NodesByIdx<'n, N, NodeIdx, DepthFirstSearch<'n, N, E, Self, Self::IncomingEdgesOf<'n>>> where Self: 'n;
    fn ancestors(&self, index: NodeIdx) -> Self::Ancestors<'_> {
        // use DFS here, Vec should be faster than VecDeque
        DepthFirstSearch::custom_ref(self, index, |graph, node| graph.incoming_edges_of(node))
    }

    type AncestorsMut<'n> = iters::NodesByIdxMut<'n, N, NodeIdx, DepthFirstSearch<'n, N, E, Self, Self::IncomingEdgesOf<'n>>> where Self: 'n;
    fn ancestors_mut(&mut self, index: NodeIdx) -> Self::AncestorsMut<'_> {
        // use DFS here, Vec should be faster than VecDeque
        DepthFirstSearch::custom_mut(self, index, |graph, node| graph.incoming_edges_of(node))
    }

    type Descendants<'n> = iters::NodesByIdx<'n, N, NodeIdx, DepthFirstSearch<'n, N, E, Self, Self::OutgoingEdgesOf<'n>>> where Self: 'n;
    fn descendants(&self, index: NodeIdx) -> Self::Descendants<'_> {
        // use DFS here, Vec should be faster than VecDeque
        DepthFirstSearch::custom_ref(self, index, |graph, node| graph.outgoing_edges_of(node))
    }

    type DescendantsMut<'n> = iters::NodesByIdxMut<'n, N, NodeIdx, DepthFirstSearch<'n, N, E, Self, Self::OutgoingEdgesOf<'n>>> where Self: 'n;
    fn descendants_mut(&mut self, index: NodeIdx) -> Self::DescendantsMut<'_> {
        // use DFS here, Vec should be faster than VecDeque
        DepthFirstSearch::custom_mut(self, index, |graph, node| graph.outgoing_edges_of(node))
    }
}
