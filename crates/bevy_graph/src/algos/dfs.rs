use hashbrown::HashSet;

use crate::graphs::{keys::NodeIdx, Graph};

/// Implementation of the [`DFS` algorythm](https://www.geeksforgeeks.org/depth-first-search-or-dfs-for-a-graph/)
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
    pub fn with_capacity(start: NodeIdx, count: usize) -> Self {
        let mut stack = Vec::with_capacity(count);
        let mut visited = HashSet::with_capacity(count);

        visited.insert(start);
        stack.push(start);

        Self { stack, visited }
    }

    /// Gets a reference to the value of the next node from the algorithm.
    pub fn next<'g, N, E>(&mut self, graph: &'g impl Graph<N, E>) -> Option<&'g N> {
        if let Some(node) = self.stack.pop() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.stack.push(idx);
                }
            }
            Some(graph.get_node(node).unwrap())
        } else {
            None
        }
    }

    /// Gets a reference to the value of the next node from the algorithm.
    ///
    /// # Safety
    ///
    /// This function should only be called when the node from the edge exists.
    /// This can happen when a node or edge gets removed but its index is still present in the DFS.
    pub unsafe fn next_unchecked<'g, N, E>(
        &mut self,
        graph: &'g impl Graph<N, E>,
    ) -> Option<&'g N> {
        if let Some(node) = self.stack.pop() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.stack.push(idx);
                }
            }
            unsafe {
                // SAFETY: the caller says its fine
                Some(graph.get_node_unchecked(node))
            }
        } else {
            None
        }
    }

    /// Gets a mutable reference to the value of the next node from the algorithm.
    pub fn next_mut<'g, N, E>(&mut self, graph: &'g mut impl Graph<N, E>) -> Option<&'g mut N> {
        if let Some(node) = self.stack.pop() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.stack.push(idx);
                }
            }
            Some(graph.get_node_mut(node).unwrap())
        } else {
            None
        }
    }

    /// Gets a mutable reference to the value of the next node from the algorythm
    ///
    /// # Safety
    ///
    /// This function should only be called when the node from the edge exists.
    /// This can happen when a node or edge gets removed but its index is still present in the DFS.
    pub unsafe fn next_unchecked_mut<'g, N, E>(
        &mut self,
        graph: &'g mut impl Graph<N, E>,
    ) -> Option<&'g mut N> {
        if let Some(node) = self.stack.pop() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.stack.push(idx);
                }
            }
            unsafe {
                // SAFETY: the caller says its fine
                Some(graph.get_node_unchecked_mut(node))
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        algos::dfs::DepthFirstSearch,
        graphs::{simple::SimpleMapGraph, Graph},
    };

    #[test]
    fn basic_imperative_dfs() {
        let mut graph = SimpleMapGraph::<i32, (), true>::new();

        let zero = graph.new_node(0);
        let one = graph.new_node(1);
        let two = graph.new_node(2);
        let three = graph.new_node(3);

        graph.new_edge(zero, one, ()).unwrap();
        graph.new_edge(zero, two, ()).unwrap();
        graph.new_edge(one, two, ()).unwrap();
        graph.new_edge(two, zero, ()).unwrap();
        graph.new_edge(two, three, ()).unwrap();

        let elements = vec![0, 1, 2, 3];

        let mut counted_elements = Vec::with_capacity(4);

        let mut dfs = DepthFirstSearch::with_capacity(zero, graph.node_count());
        while let Some(node) = dfs.next(&graph) {
            counted_elements.push(*node);
        }

        assert_eq!(elements, counted_elements);
    }
}
