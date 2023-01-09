use slotmap::{SecondaryMap, SlotMap};

use super::{EdgeIdx, NodeIdx};

pub struct SimpleListGraph<N, E, const DIRECTED: bool> {
    nodes: SlotMap<NodeIdx, N>,
    edges: SlotMap<EdgeIdx, E>,
    adjacencies: SecondaryMap<NodeIdx, Vec<(NodeIdx, EdgeIdx)>>,
}

impl<N, E, const DIRECTED: bool> SimpleListGraph<N, E, DIRECTED> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            edges: SlotMap::with_key(),
            adjacencies: SecondaryMap::new(),
        }
    }

    pub fn add_node(&mut self, node: N) -> NodeIdx {
        let idx = self.nodes.insert(node);
        self.adjacencies.insert(idx, Vec::new());
        idx
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
        let edge_idx = self
            .adjacencies
            .get(from)?
            .iter()
            .find_map(|(other_node, idx)| if *other_node == to { Some(idx) } else { None })?;
        self.edges.get(*edge_idx)
    }

    #[inline]
    pub fn edge_mut(&mut self, from: NodeIdx, to: NodeIdx) -> Option<&mut E> {
        let edge_idx = self
            .adjacencies
            .get(from)?
            .iter()
            .find_map(|(other_node, idx)| if *other_node == to { Some(idx) } else { None })?;
        self.edges.get_mut(*edge_idx)
    }
}

impl<N, E> SimpleListGraph<N, E, false> {
    pub fn add_edge(&mut self, first: NodeIdx, second: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(edge);
        self.adjacencies.get_mut(first).unwrap().push((second, idx));
        self.adjacencies.get_mut(second).unwrap().push((first, idx));
        idx // TODO: does the end user really need the idx?
    }
}

impl<N, E> SimpleListGraph<N, E, true> {
    pub fn add_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(edge);
        self.adjacencies.get_mut(from).unwrap().push((to, idx));
        idx // TODO: does the end user really need the idx?
    }
}

impl<N, E, const DIRECTED: bool> Default for SimpleListGraph<N, E, DIRECTED> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::SimpleListGraph;

    enum Person {
        Jake,
        Michael,
        Jennifer,
    }

    #[test]
    fn undirected_edge() {
        const STRENGTH: i32 = 100;

        let mut map_graph = SimpleListGraph::<Person, i32, false>::new();

        let jake = map_graph.add_node(Person::Jake);
        let michael = map_graph.add_node(Person::Michael);
        let _best_friends = map_graph.add_edge(jake, michael, STRENGTH); // TODO: does the end user really need the idx returned?

        let strength_jake = map_graph.edge(jake, michael);
        assert!(strength_jake.is_some());
        assert_eq!(strength_jake.unwrap(), &STRENGTH);

        let strength_michael = map_graph.edge(michael, jake);
        assert!(strength_michael.is_some());
        assert_eq!(strength_michael.unwrap(), &STRENGTH);
    }

    #[test]
    fn directed_edge() {
        const STRENGTH: i32 = 9999;

        let mut map_graph = SimpleListGraph::<Person, i32, true>::new();

        let jake = map_graph.add_node(Person::Jake);
        let jennifer = map_graph.add_node(Person::Jennifer);
        let _oneway_crush = map_graph.add_edge(jake, jennifer, STRENGTH); // TODO: does the end user really need the idx returned?

        let strength_jake = map_graph.edge(jake, jennifer);
        assert!(strength_jake.is_some());
        assert_eq!(strength_jake.unwrap(), &STRENGTH);

        let strength_jennifer = map_graph.edge(jennifer, jake);
        assert!(strength_jennifer.is_none());
    }
}
