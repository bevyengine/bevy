use hashbrown::HashSet;

use crate::graphs::{edge::EdgeRef, keys::NodeIdx, Graph};

/// Implementation of the [`DFS` algorithm](https://www.geeksforgeeks.org/depth-first-search-or-dfs-for-a-graph/)
///
/// it will evaluate every node from the start as deep as it can and then continue at the next sibling node from the top.
pub struct DepthFirstSearch {
    stack: Vec<NodeIdx>,
    visited: HashSet<NodeIdx>,
}

impl DepthFirstSearch {
    /// Creates a new `DepthFirstSearch` with a start node
    pub fn new(start: NodeIdx) -> Self {
        let mut stack = Vec::new();
        let mut visited = HashSet::new();

        visited.insert(start);
        stack.push(start);

        Self { stack, visited }
    }

    /// Creates a new `DepthFirstSearch` with a start node and the count of nodes for capacity reserving
    pub fn with_capacity(start: NodeIdx, node_count: usize) -> Self {
        let mut stack = Vec::with_capacity(node_count);
        let mut visited = HashSet::with_capacity(node_count);

        visited.insert(start);
        stack.push(start);

        Self { stack, visited }
    }

    /// Gets a reference to the value of the next node from the algorithm.
    pub fn next<'g, N, E>(&mut self, graph: &'g impl Graph<N, E>) -> Option<&'g N> {
        if let Some(node) = self.stack.pop() {
            for EdgeRef(_, dst, _) in graph.outgoing_edges_of(node) {
                if !self.visited.contains(&dst) {
                    self.visited.insert(dst);
                    self.stack.push(dst);
                }
            }
            Some(graph.get_node(node).unwrap())
        } else {
            None
        }
    }

    /// Gets a mutable reference to the value of the next node from the algorithm.
    pub fn next_mut<'g, N, E>(&mut self, graph: &'g mut impl Graph<N, E>) -> Option<&'g mut N> {
        if let Some(node) = self.stack.pop() {
            for EdgeRef(_, dst, _) in graph.outgoing_edges_of(node) {
                if !self.visited.contains(&dst) {
                    self.visited.insert(dst);
                    self.stack.push(dst);
                }
            }
            Some(graph.get_node_mut(node).unwrap())
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

        let mut dfs = DepthFirstSearch::with_capacity(zero, graph.node_count());
        while let Some(node) = dfs.next(&graph) {
            counted_elements.push(*node);
        }

        assert_eq!(elements, counted_elements);
    }
}
