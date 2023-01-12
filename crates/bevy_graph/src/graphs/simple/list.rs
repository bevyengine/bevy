use slotmap::{HopSlotMap, Key, SecondaryMap};

use crate::{
    error::{GraphError, GraphResult},
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
    },
    impl_graph,
};

#[derive(Clone)]
pub struct SimpleListGraph<N, E, const DIRECTED: bool> {
    nodes: HopSlotMap<NodeIdx, N>,
    edges: HopSlotMap<EdgeIdx, Edge<E>>,
    adjacencies: SecondaryMap<NodeIdx, Vec<(NodeIdx, EdgeIdx)>>,
}

impl<N, E, const DIRECTED: bool> SimpleListGraph<N, E, DIRECTED> {
    fn new() -> Self {
        Self {
            nodes: HopSlotMap::with_key(),
            edges: HopSlotMap::with_key(),
            adjacencies: SecondaryMap::new(),
        }
    }
}

impl_graph! {
    impl common for SimpleListGraph {
        #[inline]
        fn count(&self) -> usize {
            self.nodes.len()
        }

        #[inline]
        fn new_node(&mut self, node: N) -> NodeIdx {
            let idx = self.nodes.insert(node);
            self.adjacencies.insert(idx, Vec::new());
            idx
        }

        #[inline]
        fn node(&self, idx: NodeIdx) -> GraphResult<&N> {
            if let Some(node) = self.nodes.get(idx) {
                Ok(node)
            } else {
                Err(GraphError::NodeDoesntExist(idx))
            }
        }

        #[inline]
        fn node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N> {
            if let Some(node) = self.nodes.get_mut(idx) {
                Ok(node)
            } else {
                Err(GraphError::NodeDoesntExist(idx))
            }
        }

        #[inline]
        fn get_edge(&self, edge: EdgeIdx) -> Option<&E> {
            self.edges.get(edge).map(|e| &e.data)
        }

        #[inline]
        fn get_edge_mut(&mut self, edge: EdgeIdx) -> Option<&mut E> {
            self.edges.get_mut(edge).map(|e| &mut e.data)
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
        fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)> {
            self.adjacencies.get(node).unwrap().to_vec()
        }
    }

    impl undirected {
        fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N> {
            for (_, edge) in self.edges_of(node) {
                self.remove_edge(edge).unwrap();
            }
            match self.nodes.remove(node) {
                Some(n) => Ok(n),
                None => Err(GraphError::NodeDoesntExist(node))
            }
        }

        fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx {
            let idx = self.edges.insert(Edge {
                src: node,
                dst: other,
                data: edge,
            });
            self.adjacencies.get_mut(node).unwrap().push((other, idx));
            self.adjacencies.get_mut(other).unwrap().push((node, idx));
            idx
        }

        fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E> {
            if let Some((node, other)) = self.edges.get(edge).map(|e| e.indices()) {
                let list = self.adjacencies.get_mut(node).unwrap();

                if let Some(index) = find_edge(list, other) {
                    list.swap_remove(index); // TODO: remove or swap_remove ?

                    let list = self.adjacencies.get_mut(other).unwrap();
                    if let Some(index) = find_edge(list, node) {
                        list.swap_remove(index); // TODO: remove or swap_remove ?
                    }

                    Ok(self.edges.remove(edge).unwrap().data)
                } else {
                    Err(GraphError::EdgeDoesntExist(edge))
                }
            } else {
                Err(GraphError::EdgeDoesntExist(edge))
            }
        }
    }

    impl directed {
        fn remove_node(&mut self, node: NodeIdx) -> GraphResult<N> {
            let mut edges = vec![];
            for (edge, data) in &self.edges {
                let (src, dst) = data.indices();
                if dst == node || src == node {
                    edges.push(edge);
                }
            }
            for edge in edges {
                self.remove_edge(edge).unwrap();
            }
            match self.nodes.remove(node) {
                Some(n) => Ok(n),
                None => Err(GraphError::NodeDoesntExist(node))
            }
        }

        fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
            let idx = self.edges.insert(Edge {
                src: from,
                dst: to,
                data: edge,
            });
            self.adjacencies.get_mut(from).unwrap().push((to, idx));
            idx
        }

        fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E> {
            if let Some((from, to)) = self.edges.get(edge).map(|e| e.indices()) {
                let list = self.adjacencies.get_mut(from).unwrap();

                if let Some(index) = find_edge(list, to) {
                    list.swap_remove(index); // TODO: remove or swap_remove ?

                    Ok(self.edges.remove(edge).unwrap().data)
                } else {
                    Err(GraphError::EdgeDoesntExist(edge))
                }
            } else {
                Err(GraphError::EdgeDoesntExist(edge))
            }
        }
    }
}

impl<N, E, const DIRECTED: bool> Default for SimpleListGraph<N, E, DIRECTED> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// Util function
#[inline]
fn find_edge(list: &[(NodeIdx, EdgeIdx)], node: NodeIdx) -> Option<usize> {
    list.iter()
        .position(|(node_idx, _edge_idx)| *node_idx == node)
}

#[cfg(test)]
mod test {
    use slotmap::Key;

    use crate::graphs::Graph;

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
        let best_friends = list_graph.new_edge(jake, michael, STRENGTH);

        let strength_jake = list_graph
            .edge_between(jake, michael)
            .get::<Person, i32>(&list_graph);
        assert!(strength_jake.is_some());
        assert_eq!(strength_jake.unwrap(), &STRENGTH);

        let strength_michael = list_graph
            .edge_between(michael, jake)
            .get::<Person, i32>(&list_graph);
        assert!(strength_michael.is_some());
        assert_eq!(strength_michael.unwrap(), &STRENGTH);

        assert_eq!(list_graph.edges_of(jake), vec![(michael, best_friends)]);
        assert_eq!(list_graph.edges_of(michael), vec![(jake, best_friends)]);

        assert!(list_graph
            .edge_between(michael, jake)
            .remove::<Person, i32>(&mut list_graph)
            .is_ok());

        let strength_jake = list_graph
            .edge_between(jake, michael)
            .get::<Person, i32>(&list_graph);
        assert!(strength_jake.is_none());

        let strength_michael = list_graph
            .edge_between(michael, jake)
            .get::<Person, i32>(&list_graph);
        assert!(strength_michael.is_none());
    }

    #[test]
    fn directed_edge() {
        const STRENGTH: i32 = 9999;

        let mut list_graph = SimpleListGraph::<Person, i32, true>::new();

        let jake = list_graph.new_node(Person::Jake);
        let jennifer = list_graph.new_node(Person::Jennifer);
        let oneway_crush = list_graph.new_edge(jake, jennifer, STRENGTH);

        let strength_jake = list_graph
            .edge_between(jake, jennifer)
            .get::<Person, i32>(&list_graph);
        assert!(strength_jake.is_some());
        assert_eq!(strength_jake.unwrap(), &STRENGTH);

        let strength_jennifer = list_graph
            .edge_between(jennifer, jake)
            .get::<Person, i32>(&list_graph);
        assert!(strength_jennifer.is_none());

        assert_eq!(list_graph.edges_of(jake), vec![(jennifer, oneway_crush)]);
        assert_eq!(list_graph.edges_of(jennifer), vec![]);

        assert!(list_graph
            .edge_between(jake, jennifer)
            .remove::<Person, i32>(&mut list_graph)
            .is_ok());

        let strength_jake = list_graph
            .edge_between(jake, jennifer)
            .get::<Person, i32>(&list_graph);
        assert!(strength_jake.is_none());

        let strength_jennifer = list_graph
            .edge_between(jennifer, jake)
            .get::<Person, i32>(&list_graph);
        assert!(strength_jennifer.is_none());
    }

    #[test]
    fn remove_undirected_node() {
        const STRENGTH: i32 = 100;

        let mut map_graph = SimpleListGraph::<Person, i32, false>::new();

        let jake = map_graph.new_node(Person::Jake);
        let michael = map_graph.new_node(Person::Michael);

        let _best_friends = map_graph.new_edge(jake, michael, STRENGTH);

        assert!(map_graph.remove_node(michael).is_ok());

        assert!(map_graph.node(michael).is_err());
        assert!(map_graph.edge_between(jake, michael).is_null());
    }

    #[test]
    fn remove_directed_node() {
        const STRENGTH: i32 = 9999;

        let mut map_graph = SimpleListGraph::<Person, i32, true>::new();

        let jake = map_graph.new_node(Person::Jake);
        let jennifer = map_graph.new_node(Person::Jennifer);

        let _oneway_crush = map_graph.new_edge(jake, jennifer, STRENGTH);

        assert!(map_graph.remove_node(jake).is_ok());

        assert!(map_graph.node(jake).is_err());
        assert!(map_graph.edge_between(jake, jennifer).is_null());
    }
}
