use std::collections::VecDeque;

use hashbrown::HashSet;

use crate::{Graph, NodeIdx};

pub struct BreadthFirstSearch {
    queue: VecDeque<NodeIdx>,
    visited: HashSet<NodeIdx>,
}

impl BreadthFirstSearch {
    pub fn new<N, E>(start: NodeIdx, graph: &impl Graph<N, E>) -> Self {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::with_capacity(graph.len());

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
}

#[cfg(test)]
mod test {
    use crate::{graphs::simple::SimpleMapGraph, DirectedGraph, Graph};

    use super::BreadthFirstSearch;

    #[test]
    fn bfs() {
        let mut map = SimpleMapGraph::<i32, (), true>::new();

        let zero = map.new_node(0);
        let one = map.new_node(1);
        let two = map.new_node(2);
        let three = map.new_node(3);

        let sum = 0 + 1 + 2 + 3;

        map.new_edge(zero, one, ());
        map.new_edge(zero, two, ());
        map.new_edge(one, two, ());
        map.new_edge(two, zero, ());
        map.new_edge(two, three, ());

        let mut counter = 0;

        let mut bfs = BreadthFirstSearch::new(zero, &map);
        while let Some(node) = bfs.next(&map) {
            counter += node;
        }

        assert_eq!(sum, counter);
    }
}
