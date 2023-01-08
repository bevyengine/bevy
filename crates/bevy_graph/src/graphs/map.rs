use hashbrown::HashMap;
use slotmap::SlotMap;

use super::{EdgeIdx, NodeIdx};

pub struct AdjacencyMapGraph<N, E> {
    nodes: SlotMap<NodeIdx, N>,
    edges: SlotMap<EdgeIdx, E>,
    adjacencies: HashMap<NodeIdx, HashMap<NodeIdx, EdgeIdx>>,
}

impl<N, E> AdjacencyMapGraph<N, E> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::new(),
            edges: SlotMap::new(),
            adjacencies: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: N) -> NodeIdx {
        let idx = self.nodes.insert(node);
        self.adjacencies.insert(idx, HashMap::new());
        idx
    }

    pub fn add_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(edge);
        self.adjacencies.get_mut(&from).unwrap().insert(to, idx);
        idx // TODO: does the end user really need the idx?
    }

    #[inline]
    pub fn node(&self, idx: NodeIdx) -> Option<&N> {
        self.nodes.get(idx)
    }

    #[inline]
    pub fn node_mut(&mut self, idx: NodeIdx) -> Option<&mut N> {
        self.nodes.get_mut(idx)
    }

    #[inline]
    pub fn edge(&self, from: NodeIdx, to: NodeIdx) -> Option<&E> {
        let edge_idx = self.adjacencies.get(&from)?.get(&to)?;
        self.edges.get(*edge_idx)
    }

    #[inline]
    pub fn edge_mut(&mut self, from: NodeIdx, to: NodeIdx) -> Option<&mut E> {
        let edge_idx = self.adjacencies.get(&from)?.get(&to)?;
        self.edges.get_mut(*edge_idx)
    }
}

impl<N, E> Default for AdjacencyMapGraph<N, E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::AdjacencyMapGraph;

    enum Person {
        Jake,
        Michael,
    }

    #[test]
    fn get_edge() {
        const STRENGTH: i32 = 100;

        let mut map_graph = AdjacencyMapGraph::<Person, i32>::new();

        let jake = map_graph.add_node(Person::Jake);
        let michael = map_graph.add_node(Person::Michael);
        let _best_friends = map_graph.add_edge(jake, michael, STRENGTH); // TODO: does the end user really need the idx returned?

        let stength = map_graph.edge(jake, michael);
        assert!(stength.is_some());
        assert_eq!(stength.unwrap(), &STRENGTH);
    }
}
