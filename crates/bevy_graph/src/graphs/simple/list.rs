use slotmap::{HopSlotMap, Key, SecondaryMap};

use crate::{
    error::{GraphError, GraphResult},
    graphs::{
        edge::Edge,
        keys::{EdgeIdx, NodeIdx},
        Graph,
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
    pub fn new() -> Self {
        Self {
            nodes: HopSlotMap::with_key(),
            edges: HopSlotMap::with_key(),
            adjacencies: SecondaryMap::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline]
    pub fn new_node(&mut self, node: N) -> NodeIdx {
        let idx = self.nodes.insert(node);
        self.adjacencies.insert(idx, Vec::new());
        idx
    }

    #[inline]
    pub fn node(&self, idx: NodeIdx) -> GraphResult<&N> {
        if let Some(node) = self.nodes.get(idx) {
            Ok(node)
        } else {
            Err(GraphError::NodeDoesntExist(idx))
        }
    }

    #[inline]
    pub fn node_mut(&mut self, idx: NodeIdx) -> GraphResult<&mut N> {
        if let Some(node) = self.nodes.get_mut(idx) {
            Ok(node)
        } else {
            Err(GraphError::NodeDoesntExist(idx))
        }
    }

    #[inline]
    pub fn get_edge(&self, edge: EdgeIdx) -> Option<&E> {
        self.edges.get(edge).map(|e| &e.data)
    }

    #[inline]
    pub fn get_edge_mut(&mut self, edge: EdgeIdx) -> Option<&mut E> {
        self.edges.get_mut(edge).map(|e| &mut e.data)
    }

    #[inline]
    pub fn edge_between(&self, from: NodeIdx, to: NodeIdx) -> EdgeIdx {
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
    pub fn edges_of(&self, node: NodeIdx) -> Vec<(NodeIdx, EdgeIdx)> {
        self.adjacencies.get(node).unwrap().to_vec()
    }
}

impl<N, E> SimpleListGraph<N, E, false> {
    pub fn new_edge(&mut self, node: NodeIdx, other: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(Edge {
            src: node,
            dst: other,
            data: edge,
        });
        self.adjacencies.get_mut(node).unwrap().push((other, idx));
        self.adjacencies.get_mut(other).unwrap().push((node, idx));
        idx
    }

    pub fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E> {
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

impl<N, E> SimpleListGraph<N, E, true> {
    pub fn new_edge(&mut self, from: NodeIdx, to: NodeIdx, edge: E) -> EdgeIdx {
        let idx = self.edges.insert(Edge {
            src: from,
            dst: to,
            data: edge,
        });
        self.adjacencies.get_mut(from).unwrap().push((to, idx));
        idx
    }

    pub fn remove_edge(&mut self, edge: EdgeIdx) -> GraphResult<E> {
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

impl_graph!(SimpleListGraph, false);
impl_graph!(SimpleListGraph, true);

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
}
