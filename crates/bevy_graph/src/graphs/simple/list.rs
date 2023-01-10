use slotmap::{HopSlotMap, Key, SecondaryMap};

use crate::{DirectedGraph, EdgeIdx, Graph, NodeIdx, UndirectedGraph};

#[derive(Clone)]
pub struct SimpleListGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, E>,
    adjacencies: SecondaryMap<NodeIdx, Vec<(NodeIdx, EdgeIdx)>>,
}

impl<N, E, const DIRECTED: bool> SimpleListGraph<N, E, DIRECTED> {
    pub fn new() -> Self {
        Self {
            nodes: HopSlotMap::with_key(),
            edges: HopSlotMap::with_key(),
            adjacencies: SecondaryMap::new(),
        }
    }
}

impl<N, E, const DIRECTED: bool> Graph<N, E> for SimpleListGraph<N, E, DIRECTED> {
    fn new_node(&mut self, node: N) -> NodeIdx {
        let idx = self.nodes.insert(node);
        self.adjacencies.insert(idx, Vec::new());
        idx
    }

    #[inline]
    fn node(&self, idx: NodeIdx) -> Option<&N> {
        self.nodes.get(idx)
    }

    #[inline]
    fn node_mut(&mut self, idx: NodeIdx) -> Option<&mut N> {
        self.nodes.get_mut(idx)
    }

    #[inline]
    fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx {
        if let Some(idx) = self
            .adjacencies
            .get(from)
            .unwrap()
            .iter()
            .find_map(|(other_node, idx)| if *other_node == to { Some(*idx) } else { None })
        {
            idx
        } else {
            EdgeIdx::null()
        }
    }

    #[inline]
    fn get_edge(&self, edge: EdgeIdx) -> Option<&E> {
        self.edges.get(edge)
    }

    #[inline]
    fn get_edge_mut(&mut self, edge: EdgeIdx) -> Option<&mut E> {
        self.edges.get_mut(edge)
    }
}

impl<N, E> UndirectedGraph<N, E> for SimpleListGraph<N, E, false> {
    fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(edge);
        self.adjacencies.get_mut(node).unwrap().push((other, idx));
        self.adjacencies.get_mut(other).unwrap().push((node, idx));
        idx
    }

    fn remove_edge_between(&mut self, node: NodeIdx, other: NodeIdx) -> Option<E> {
        let list = self.adjacencies.get_mut(node)?;

        if let Some(index) = list
            .iter()
            .position(|(node_idx, _edge_idx)| *node_idx == other)
        {
            let (_, edge_idx) = list.swap_remove(index); // TODO: remove or swap_remove ?

            let list = self.adjacencies.get_mut(other)?;
            if let Some(index) = list.iter().position(|(node_idx, _)| *node_idx == node) {
                list.swap_remove(index); // TODO: remove or swap_remove ?
            }

            self.edges.remove(edge_idx)
        } else {
            None
        }
    }
}

impl<N, E> DirectedGraph<N, E> for SimpleListGraph<N, E, true> {
    fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(edge);
        self.adjacencies.get_mut(from).unwrap().push((to, idx));
        idx
    }

    fn remove_edge_between(&mut self, from: NodeIdx, to: NodeIdx) -> Option<E> {
        let list = self.adjacencies.get_mut(from).unwrap();

        if let Some(index) = list.iter().position(|(node_idx, _)| *node_idx == to) {
            let (_, edge_idx) = list.swap_remove(index); // TODO: remove or swap_remove ?

            self.edges.remove(edge_idx)
        } else {
            None
        }
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
    use crate::{DirectedGraph, Graph, UndirectedGraph};

    use super::SimpleListGraph;

    enum Person {
        Jake,
        Michael,
        Jennifer,
    }

    #[test]
    fn undirected_edge() {
        const STRENGTH: i32 = 100;

        let mut list_graph = SimpleListGraph::<Person, i32, false>::new();

        let jake = list_graph.new_node(Person::Jake);
        let michael = list_graph.new_node(Person::Michael);
        let _best_friends = list_graph.new_edge(jake, michael, STRENGTH);

        let strength_jake = list_graph.edge_between(jake, michael).get(&list_graph);
        assert!(strength_jake.is_some());
        assert_eq!(strength_jake.unwrap(), &STRENGTH);

        let strength_michael = list_graph.edge_between(michael, jake).get(&list_graph);
        assert!(strength_michael.is_some());
        assert_eq!(strength_michael.unwrap(), &STRENGTH);

        list_graph.remove_edge_between(michael, jake);

        let strength_jake = list_graph.edge_between(jake, michael).get(&list_graph);
        assert!(strength_jake.is_none());

        let strength_michael = list_graph.edge_between(michael, jake).get(&list_graph);
        assert!(strength_michael.is_none());
    }

    #[test]
    fn directed_edge() {
        const STRENGTH: i32 = 9999;

        let mut list_graph = SimpleListGraph::<Person, i32, true>::new();

        let jake = list_graph.new_node(Person::Jake);
        let jennifer = list_graph.new_node(Person::Jennifer);
        let _oneway_crush = list_graph.new_edge(jake, jennifer, STRENGTH);

        let strength_jake = list_graph.edge_between(jake, jennifer).get(&list_graph);
        assert!(strength_jake.is_some());
        assert_eq!(strength_jake.unwrap(), &STRENGTH);

        let strength_jennifer = list_graph.edge_between(jennifer, jake).get(&list_graph);
        assert!(strength_jennifer.is_none());

        list_graph.remove_edge_between(jake, jennifer);

        let strength_jake = list_graph.edge_between(jake, jennifer).get(&list_graph);
        assert!(strength_jake.is_none());

        let strength_jennifer = list_graph.edge_between(jennifer, jake).get(&list_graph);
        assert!(strength_jennifer.is_none());
    }
}
