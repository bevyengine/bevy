use std::{collections::VecDeque, marker::PhantomData};

use hashbrown::HashSet;

use crate::{
    graphs::{edge::EdgeRef, keys::NodeIdx, Graph},
    iters,
};

/// Implementation of the [`BFS` algorithm](https://www.geeksforgeeks.org/breadth-first-search-or-bfs-for-a-graph/)
///
/// when `d` is the distance between a node and the startnode,
/// it will evaluate every node with `d=1`, then continue with `d=2` and so on.
pub struct BreadthFirstSearch<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = EdgeRef<'g, E>>> {
    graph: &'g G,
    queue: VecDeque<NodeIdx>,
    visited: HashSet<NodeIdx>,
    visitor: fn(&'g G, NodeIdx) -> I,
    phantom: PhantomData<(N, E)>,
}

impl<'g, N, E: 'g, G: Graph<N, E>, I: Iterator<Item = EdgeRef<'g, E>>>
    BreadthFirstSearch<'g, N, E, G, I>
{
    /// Creates a new `BreadthFirstSearch` with a start node and a custom visitor
    pub fn custom(graph: &'g G, start: NodeIdx, visitor: fn(&'g G, NodeIdx) -> I) -> Self {
        let node_count = graph.node_count();
        let mut queue = VecDeque::with_capacity(node_count);
        let mut visited = HashSet::with_capacity(node_count);

        visited.insert(start);
        queue.push_back(start);

        Self {
            graph,
            queue,
            visited,
            visitor,
            phantom: PhantomData,
        }
    }
}

impl<'g, N, E: 'g, G: Graph<N, E>> BreadthFirstSearch<'g, N, E, G, G::OutgoingEdgesOf<'g>> {
    /// Creates a new `BreadthFirstSearch` with a start node and the default visitor of `outgoing`
    #[inline]
    pub fn new(graph: &'g G, start: NodeIdx) -> Self {
        Self::custom(graph, start, |graph, index| graph.outgoing_edges_of(index))
    }

    /// Creates a new `BreadthFirstSearch` with a start node and the default visitor of `outgoing` wrapped inside an `NodesByIdx` iterator
    #[inline]
    pub fn new_ref(graph: &'g G, start: NodeIdx) -> iters::NodesByIdx<'g, N, NodeIdx, Self> {
        let inner = Self::new(graph, start);
        iters::NodesByIdx::from_graph(inner, graph)
    }

    /// Creates a new `BreadthFirstSearch` with a start node and the default visitor of `outgoing` wrapped inside an `NodesByIdxMut` iterator
    #[inline]
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
    for BreadthFirstSearch<'g, N, E, G, I>
{
    type Item = NodeIdx;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.queue.pop_front() {
            for EdgeRef(_, dst, _) in (self.visitor)(self.graph, node) {
                if !self.visited.contains(&dst) {
                    self.visited.insert(dst);
                    self.queue.push_back(dst);
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
        algos::bfs::BreadthFirstSearch,
        graphs::{map::SimpleMapGraph, Graph},
    };

    #[test]
    fn basic_imperative_bfs() {
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

        let elements = vec![0, 2, 1, 3];

        let mut counted_elements = Vec::with_capacity(4);

        let bfs = BreadthFirstSearch::new_ref(&graph, zero);
        for node in bfs {
            counted_elements.push(*node);
        }

        assert_eq!(elements, counted_elements);
    }
}
