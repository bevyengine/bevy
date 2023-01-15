mod map;
pub use map::*;

mod list;
pub use list::*;

#[cfg(test)]
mod test {
    use hashbrown::HashSet;
    use std::hash::Hash;

    use crate::{error::GraphError, graphs::SimpleGraph};

    #[derive(PartialEq, Debug)]
    pub enum Person {
        Jake,
        Michael,
        Jennifer,
    }

    #[macro_export]
    macro_rules! simple_graph_tests {
        ($($graph:ident )::+) => {
            use $crate::graphs::simple::test::{self, Person};

            #[test]
            fn nodes() {
                test::nodes(<$($graph)::+ <Person, i32, false>>::new())
            }
            #[test]
            fn undirected_edges() {
                test::undirected_edges(<$($graph)::+ <Person, i32, false>>::new())
            }
            #[test]
            fn directed_edges() {
                test::directed_edges(<$($graph)::+ <Person, i32, true>>::new())
            }
            #[test]
            fn remove_node_undirected() {
                test::remove_node_undirected(<$($graph)::+ <Person, i32, false>>::new())
            }
            #[test]
            fn remove_node_directed() {
                test::remove_node_directed(<$($graph)::+ <Person, i32, true>>::new())
            }
            #[test]
            fn edge_between_same_node_undirected() {
                test::edge_between_same_node(<$($graph)::+ <Person, i32, false>>::new())
            }
            #[test]
            fn edge_between_same_node_directed() {
                test::edge_between_same_node(<$($graph)::+ <Person, i32, true>>::new())
            }
        };
    }

    pub fn nodes(mut graph: impl SimpleGraph<Person, i32>) {
        let jake = graph.new_node(Person::Jake);
        let michael = graph.new_node(Person::Michael);
        let jennifer = graph.new_node(Person::Jennifer);
        let other_jake = graph.new_node(Person::Jake);

        assert_eq!(graph.get_node(jake).unwrap(), &Person::Jake);
        assert_eq!(graph.get_node(michael).unwrap(), &Person::Michael);
        assert_eq!(graph.get_node(jennifer).unwrap(), &Person::Jennifer);
        assert_eq!(graph.get_node(other_jake).unwrap(), &Person::Jake);

        graph
            .get_node_mut(jake)
            .map(|node| *node = Person::Michael)
            .unwrap();

        assert_eq!(graph.get_node(jake).unwrap(), &Person::Michael);
        assert_eq!(graph.get_node(michael).unwrap(), &Person::Michael);
        assert_eq!(graph.get_node(jennifer).unwrap(), &Person::Jennifer);
        assert_eq!(graph.get_node(other_jake).unwrap(), &Person::Jake);

        assert!(graph.remove_node(jake).is_ok());
        assert!(graph.remove_node(michael).is_ok());
        assert!(graph.remove_node(jennifer).is_ok());
        assert!(graph.remove_node(other_jake).is_ok());

        assert!(graph.get_node(jake).is_err());
        assert!(graph.get_node(michael).is_err());
        assert!(graph.get_node(jennifer).is_err());
        assert!(graph.get_node(other_jake).is_err());
    }

    pub fn undirected_edges(mut graph: impl SimpleGraph<Person, i32>) {
        let jake = graph.new_node(Person::Jake);
        let michael = graph.new_node(Person::Michael);
        let jennifer = graph.new_node(Person::Jennifer);
        let other_jake = graph.new_node(Person::Jake);

        let jm = graph.new_edge(jake, michael, 2).unwrap();
        let jj = graph.new_edge(jennifer, jake, 7).unwrap();
        let jo = graph.new_edge(jake, other_jake, 5).unwrap();
        let mo = graph.new_edge(michael, other_jake, 1).unwrap();

        assert!(unordered_eq(
            &graph.edges_of(jake),
            &[(michael, jm), (jennifer, jj), (other_jake, jo)]
        ));

        assert_eq!(graph.get_edge(jm).unwrap(), &2);
        assert_eq!(graph.get_edge(jj).unwrap(), &7);
        assert_eq!(graph.get_edge(jo).unwrap(), &5);
        assert_eq!(graph.get_edge(mo).unwrap(), &1);

        assert_eq!(
            graph.edge_between(jennifer, jake).unwrap(),
            graph.edge_between(jake, jennifer).unwrap()
        );

        *graph.get_edge_mut(mo).unwrap() = 10;

        assert_eq!(graph.get_edge(jm).unwrap(), &2);
        assert_eq!(graph.get_edge(jj).unwrap(), &7);
        assert_eq!(graph.get_edge(jo).unwrap(), &5);
        assert_eq!(graph.get_edge(mo).unwrap(), &10);

        assert!(graph.remove_edge(jm).is_ok());
        assert!(graph.remove_edge(jj).is_ok());
        assert!(graph.remove_edge(jo).is_ok());
        assert!(graph.remove_edge(mo).is_ok());

        assert!(graph.get_edge(jm).is_err());
        assert!(graph.get_edge(jj).is_err());
        assert!(graph.get_edge(jo).is_err());
        assert!(graph.get_edge(mo).is_err());
    }

    pub fn directed_edges(mut graph: impl SimpleGraph<Person, i32>) {
        let jake = graph.new_node(Person::Jake);
        let michael = graph.new_node(Person::Michael);
        let jennifer = graph.new_node(Person::Jennifer);
        let other_jake = graph.new_node(Person::Jake);

        let jm = graph.new_edge(jake, michael, 2).unwrap();
        let jj = graph.new_edge(jennifer, jake, 7).unwrap();
        let jo = graph.new_edge(jake, other_jake, 5).unwrap();
        let mo = graph.new_edge(michael, other_jake, 1).unwrap();

        assert!(unordered_eq(
            &graph.edges_of(jake),
            &[(michael, jm), (other_jake, jo)]
        ));

        assert_eq!(graph.get_edge(jm).unwrap(), &2);
        assert_eq!(graph.get_edge(jj).unwrap(), &7);
        assert_eq!(graph.get_edge(jo).unwrap(), &5);
        assert_eq!(graph.get_edge(mo).unwrap(), &1);

        assert!(graph.edge_between(jennifer, jake).is_ok());
        assert!(graph.edge_between(jake, jennifer).is_err());

        *graph.get_edge_mut(mo).unwrap() = 10;

        assert_eq!(graph.get_edge(jm).unwrap(), &2);
        assert_eq!(graph.get_edge(jj).unwrap(), &7);
        assert_eq!(graph.get_edge(jo).unwrap(), &5);
        assert_eq!(graph.get_edge(mo).unwrap(), &10);

        assert!(graph.remove_edge(jm).is_ok());
        assert!(graph.remove_edge(jj).is_ok());
        assert!(graph.remove_edge(jo).is_ok());
        assert!(graph.remove_edge(mo).is_ok());

        assert!(graph.get_edge(jm).is_err());
        assert!(graph.get_edge(jj).is_err());
        assert!(graph.get_edge(jo).is_err());
        assert!(graph.get_edge(mo).is_err());
    }

    pub fn remove_node_undirected(mut graph: impl SimpleGraph<Person, i32>) {
        let jake = graph.new_node(Person::Jake);
        let michael = graph.new_node(Person::Michael);

        let edge = graph.new_edge(jake, michael, 20).unwrap();

        assert!(graph.get_node(jake).is_ok());
        assert!(graph.get_node(michael).is_ok());
        assert_eq!(graph.get_edge(edge).unwrap(), &20);
        assert_eq!(
            graph
                .edge_between(jake, michael)
                .unwrap()
                .get(&graph)
                .unwrap(),
            &20
        );
        assert_eq!(
            graph
                .edge_between(michael, jake)
                .unwrap()
                .get(&graph)
                .unwrap(),
            &20
        );

        assert!(graph.remove_node(michael).is_ok());

        assert!(graph.get_node(jake).is_ok());
        assert!(graph.get_node(michael).is_err());
        assert!(graph.get_edge(edge).is_err());
        assert!(graph.edge_between(jake, michael).is_err());
        assert!(graph.edge_between(michael, jake).is_err());
    }

    pub fn remove_node_directed(mut graph: impl SimpleGraph<Person, i32>) {
        let jake = graph.new_node(Person::Jake);
        let michael = graph.new_node(Person::Michael);

        let edge = graph.new_edge(jake, michael, 20).unwrap();

        assert!(graph.get_node(jake).is_ok());
        assert!(graph.get_node(michael).is_ok());
        assert_eq!(graph.get_edge(edge).unwrap(), &20);
        assert_eq!(
            graph
                .edge_between(jake, michael)
                .unwrap()
                .get(&graph)
                .unwrap(),
            &20
        );
        assert!(graph.edge_between(michael, jake).is_err());

        assert!(graph.remove_node(michael).is_ok());

        assert!(graph.get_node(jake).is_ok());
        assert!(graph.get_node(michael).is_err());
        assert!(graph.get_edge(edge).is_err());
        assert!(graph.edge_between(jake, michael).is_err());
        assert!(graph.edge_between(michael, jake).is_err());
    }

    pub fn edge_between_same_node(mut graph: impl SimpleGraph<Person, i32>) {
        let jake = graph.new_node(Person::Jake);

        assert!(matches!(
            graph.new_edge(jake, jake, 20),
            Err(GraphError::EdgeBetweenSameNode(node)) if node == jake
        ));
    }

    fn unordered_eq<T>(a: &[T], b: &[T]) -> bool
    where
        T: Eq + Hash,
    {
        let a: HashSet<_> = a.iter().collect();
        let b: HashSet<_> = b.iter().collect();

        a == b
    }
}
