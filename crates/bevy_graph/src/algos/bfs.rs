use std::collections::VecDeque;

use hashbrown::HashSet;

use crate::{Graph, NodeIdx};

pub fn breadth_first_search<N, E>(start: NodeIdx, graph: &impl Graph<N, E>, visitor: fn(&N)) {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::with_capacity(graph.len());

    visited.insert(start);
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        visitor(graph.node(node).unwrap());
        for (idx, _) in graph.edges_of(node) {
            if !visited.contains(&idx) {
                visited.insert(idx);
                queue.push_back(idx);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{graphs::simple::SimpleMapGraph, DirectedGraph, Graph};

    use super::breadth_first_search;

    #[test]
    fn bfs() {
        let mut map = SimpleMapGraph::<i32, (), true>::new();

        let zero = map.new_node(0);
        let one = map.new_node(1);
        let two = map.new_node(2);
        let three = map.new_node(3);

        map.new_edge(zero, one, ());
        map.new_edge(zero, two, ());
        map.new_edge(one, two, ());
        map.new_edge(two, zero, ());
        map.new_edge(two, three, ());

        breadth_first_search(zero, &map, |value| println!("{value}"));
    }
}
