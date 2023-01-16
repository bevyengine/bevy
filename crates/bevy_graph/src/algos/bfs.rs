use std::collections::VecDeque;

use hashbrown::HashSet;

use crate::graphs::{keys::NodeIdx, Graph};

/// Implementation of the [`BFS` algorythm](https://www.geeksforgeeks.org/breadth-first-search-or-bfs-for-a-graph/)
///
/// when `d` is the distance between a node and the startnode,
/// it will evaluate every node with `d=1`, then continue with `d=2` and so on.
pub struct BreadthFirstSearch {
    queue: VecDeque<NodeIdx>,
    visited: HashSet<NodeIdx>,
}

impl BreadthFirstSearch {
    /// Creates a new `DepthFirstSearch` with a start node and the count of nodes
    pub fn new(start: NodeIdx, count: usize) -> Self {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::with_capacity(count);

        visited.insert(start);
        queue.push_back(start);

        Self { queue, visited }
    }

    /// Gets a reference to the value of the next node from the algorythm
    pub fn next<'g, N, E>(&mut self, graph: &'g impl Graph<N, E>) -> Option<&'g N> {
        if let Some(node) = self.queue.pop_front() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.queue.push_back(idx);
                }
            }
            Some(graph.get_node(node).unwrap())
        } else {
            None
        }
    }

    /// Gets a reference to the value of the next node from the algorythm
    ///
    /// # Safety
    ///
    /// This function should only be called when the node from the edge exists.
    /// This can happen when a node or edge gets removed but its index is still present in the BFS.
    pub unsafe fn next_unchecked<'g, N, E>(
        &mut self,
        graph: &'g impl Graph<N, E>,
    ) -> Option<&'g N> {
        if let Some(node) = self.queue.pop_front() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.queue.push_back(idx);
                }
            }
            Some(graph.get_node_unchecked(node))
        } else {
            None
        }
    }

    /// Gets a mutable reference to the value of the next node from the algorythm
    pub fn next_mut<'g, N, E>(&mut self, graph: &'g mut impl Graph<N, E>) -> Option<&'g mut N> {
        if let Some(node) = self.queue.pop_front() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.queue.push_back(idx);
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
    /// This can happen when a node or edge gets removed but its index is still present in the BFS.
    pub unsafe fn next_unchecked_mut<'g, N, E>(
        &mut self,
        graph: &'g mut impl Graph<N, E>,
    ) -> Option<&'g mut N> {
        if let Some(node) = self.queue.pop_front() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.queue.push_back(idx);
                }
            }
            Some(graph.get_node_unchecked_mut(node))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::graphs::{simple::SimpleMapGraph, Graph};

    #[test]
    fn bfs() {
        let mut map = SimpleMapGraph::<i32, (), true>::new();

        let zero = map.new_node(0);
        let one = map.new_node(1);
        let two = map.new_node(2);
        let three = map.new_node(3);

        map.new_edge(zero, one, ()).unwrap();
        map.new_edge(zero, two, ()).unwrap();
        map.new_edge(one, two, ()).unwrap();
        map.new_edge(two, zero, ()).unwrap();
        map.new_edge(two, three, ()).unwrap();

        let elements = vec![0, 2, 1, 3];

        let mut counted_elements = Vec::with_capacity(4);

        let mut bfs = map.algo_bfs(zero);
        while let Some(node) = bfs.next(&map) {
            counted_elements.push(*node);
        }

        assert_eq!(elements, counted_elements);
    }
}
