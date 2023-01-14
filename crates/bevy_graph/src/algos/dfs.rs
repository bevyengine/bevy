use hashbrown::HashSet;

use crate::graphs::{keys::NodeIdx, Graph};

pub struct DepthFirstSearch {
    stack: Vec<NodeIdx>,
    visited: HashSet<NodeIdx>,
}

impl DepthFirstSearch {
    pub fn new(start: NodeIdx, count: usize) -> Self {
        let mut stack = Vec::new();
        let mut visited = HashSet::with_capacity(count);

        visited.insert(start);
        stack.push(start);

        Self { stack, visited }
    }

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
}

#[cfg(test)]
mod test {
    use crate::graphs::{simple::SimpleMapGraph, Graph};

    #[test]
    fn dfs() {
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

        let elements = vec![0, 1, 2, 3];

        let mut counted_elements = Vec::with_capacity(4);

        let mut dfs = map.algo_dfs(zero);
        while let Some(node) = dfs.next(&map) {
            counted_elements.push(*node)
        }

        assert_eq!(elements, counted_elements);
    }
}
