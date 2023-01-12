use std::collections::VecDeque;

use hashbrown::HashSet;

use crate::graphs::{keys::NodeIdx, Graph};

pub struct BreadthFirstSearch {
    queue: VecDeque<NodeIdx>,
    visited: HashSet<NodeIdx>,
}

impl BreadthFirstSearch {
    pub fn new(start: NodeIdx, count: usize) -> Self {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::with_capacity(count);

        visited.insert(start);
        queue.push_back(start);

        Self { queue, visited }
    }

    pub fn next<'g, N, E>(&mut self, graph: &'g impl Graph<N, E>) -> Option<&'g N> {
        if let Some(node) = self.queue.pop_front() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.queue.push_back(idx);
                }
            }
            Some(graph.node(node).unwrap())
        } else {
            None
        }
    }

    pub fn next_mut<'g, N, E>(&mut self, graph: &'g mut impl Graph<N, E>) -> Option<&'g mut N> {
        if let Some(node) = self.queue.pop_front() {
            for (idx, _) in graph.edges_of(node) {
                if !self.visited.contains(&idx) {
                    self.visited.insert(idx);
                    self.queue.push_back(idx);
                }
            }
            Some(graph.node_mut(node).unwrap())
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

        let sum = 6; // 0 + 1 + 2 + 3

        map.new_edge(zero, one, ());
        map.new_edge(zero, two, ());
        map.new_edge(one, two, ());
        map.new_edge(two, zero, ());
        map.new_edge(two, three, ());

        let mut counter = 0;

        let mut bfs = map.algo_bfs(zero);
        while let Some(node) = bfs.next(&map) {
            counter += node;
        }

        assert_eq!(sum, counter);
    }
}
