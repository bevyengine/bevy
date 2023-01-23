use std::marker::PhantomData;

use hashbrown::HashSet;

use crate::{
    graphs::{edge::EdgeRef, keys::NodeIdx, Graph},
    iters,
};

/// Implementation of the [`DFS` algorithm](https://www.geeksforgeeks.org/depth-first-search-or-dfs-for-a-graph/)
///
/// it will evaluate every node from the start as deep as it can and then continue at the next sibling node from the top.
pub struct DepthFirstSearch<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = EdgeRef<'g, E>>> {
    graph: &'g G,
    stack: Vec<NodeIdx>,
    visited: HashSet<NodeIdx>,
    visitor: fn(&'g G, NodeIdx) -> I,
    phantom: PhantomData<(N, E)>,
}

impl<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = EdgeRef<'g, E>>>
    DepthFirstSearch<'g, N, E, G, I>
{
    /// Creates a new `DepthFirstSearch` with a start node and a custom visitor
    pub fn custom(graph: &'g G, start: NodeIdx, visitor: fn(&'g G, NodeIdx) -> I) -> Self {
        let node_count = graph.node_count();
        let mut stack = Vec::with_capacity(node_count);
        let mut visited = HashSet::with_capacity(node_count);

        visited.insert(start);
        stack.push(start);

        Self {
            graph,
            stack,
            visited,
            visitor,
            phantom: PhantomData,
        }
    }

    /// Creates a new `DepthFirstSearch` with a start node and a custom visitor wrapped inside an `NodesByIdx` iterator
    #[inline]
    pub fn custom_ref(
        graph: &'g G,
        start: NodeIdx,
        visitor: fn(&'g G, NodeIdx) -> I,
    ) -> iters::NodesByIdx<'g, N, NodeIdx, Self> {
        let inner = Self::custom(graph, start, visitor);
        iters::NodesByIdx::from_graph(inner, graph)
    }

    /// Creates a new `DepthFirstSearch` with a start node and a custom visitor wrapped inside an `NodesByIdxMut` iterator
    #[inline]
    pub fn custom_mut(
        graph: &'g mut G,
        start: NodeIdx,
        visitor: fn(&'g G, NodeIdx) -> I,
    ) -> iters::NodesByIdxMut<'g, N, NodeIdx, Self> {
        unsafe {
            // SAFETY: We know `NodesByIdxMut` doesn't intercept (deletes nodes) at all.
            let ptr: *mut G = &mut *graph;
            let inner = Self::custom(&*ptr, start, visitor);

            iters::NodesByIdxMut::from_graph(inner, graph)
        }
    }
}

impl<'g, N, E: 'g, G: Graph<N, E>> DepthFirstSearch<'g, N, E, G, G::OutgoingEdgesOf<'g>> {
    /// Creates a new `DepthFirstSearch` with a start node and the default visitor of `outgoing`
    #[inline]
    pub fn new(graph: &'g G, start: NodeIdx) -> Self {
        Self::custom(graph, start, |graph, index| graph.outgoing_edges_of(index))
    }

    /// Creates a new `DepthFirstSearch` with a start node and the default visitor of `outgoing` wrapped inside an `NodesByIdx` iterator
    pub fn new_ref(graph: &'g G, start: NodeIdx) -> iters::NodesByIdx<'g, N, NodeIdx, Self> {
        let inner = Self::new(graph, start);
        iters::NodesByIdx::from_graph(inner, graph)
    }

    /// Creates a new `DepthFirstSearch` with a start node and the default visitor of `outgoing` wrapped inside an `NodesByIdxMut` iterator
    pub fn new_mut(graph: &'g mut G, start: NodeIdx) -> iters::NodesByIdxMut<'g, N, NodeIdx, Self> {
        unsafe {
            // SAFETY: We know `NodesByIdxMut` doesn't intercept (deletes nodes) at all.
            let ptr: *mut G = &mut *graph;
            let inner = Self::new(&*ptr, start);

            iters::NodesByIdxMut::from_graph(inner, graph)
        }
    }
}

impl<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = EdgeRef<'g, E>>> Iterator
    for DepthFirstSearch<'g, N, E, G, I>
{
    type Item = NodeIdx;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            for EdgeRef(_, dst, _) in (self.visitor)(self.graph, node) {
                if !self.visited.contains(&dst) {
                    self.visited.insert(dst);
                    self.stack.push(dst);
                }
            }
            Some(node)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        algos::dfs::DepthFirstSearch,
        graphs::{map::SimpleMapGraph, Graph},
    };

    #[test]
    fn basic_imperative_dfs() {
        let mut graph = SimpleMapGraph::<i32, (), true>::new();

        let zero = graph.add_node(0);
        let one = graph.add_node(1);
        let two = graph.add_node(2);
        let three = graph.add_node(3);

        graph.add_edge(zero, one, ());
        graph.add_edge(zero, two, ());
        graph.add_edge(one, two, ());
        graph.add_edge(two, zero, ());
        graph.add_edge(two, three, ());

        let elements = vec![0, 1, 2, 3];

        let mut counted_elements = Vec::with_capacity(4);

        let dfs = DepthFirstSearch::new_ref(&graph, zero);
        for node in dfs {
            counted_elements.push(*node);
        }

        assert_eq!(elements, counted_elements);
    }
}
