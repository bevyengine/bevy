use bevy_graph::{
    graphs::{
        keys::{EdgeIdx, NodeIdx},
        map::SimpleMapGraph,
        Graph,
    },
    utils::wrapped_iterator::WrappedIterator,
};
use hashbrown::HashSet;

#[test]
fn undirected() {
    let mut graph = SimpleMapGraph::<&str, i32, false>::new();

    assert!(!graph.is_directed());
    assert!(!graph.is_multigraph());

    assert_eq!(graph.node_count(), 0);
    let jakob = graph.add_node("Jakob");
    let edgar = graph.add_node("Edgar");
    let bernhard = graph.add_node("Bernhard");
    assert_eq!(graph.node_count(), 3);

    assert!(graph.contains_node(jakob));
    assert!(graph.contains_node(edgar));
    assert!(graph.contains_node(bernhard));

    assert_eq!(graph.find_node(&"Edgar"), Some(edgar));
    assert_eq!(graph.find_node(&"NoIReallyDon'tExist"), None);

    assert_eq!(
        &graph.node_indices().collect::<HashSet<NodeIdx>>(),
        &[jakob, edgar, bernhard].into()
    );

    assert_eq!(graph.edge_count(), 0);
    let je = graph.add_edge(jakob, edgar, 12);
    let eb = graph.add_edge(edgar, bernhard, 7);
    assert_eq!(graph.edge_count(), 2);

    assert!(graph.contains_edge_between(jakob, edgar));
    assert!(graph.contains_edge_between(edgar, jakob));
    assert!(!graph.contains_edge_between(jakob, bernhard));

    assert_eq!(graph.find_edge(&12), Some(je));
    assert_eq!(graph.find_edge(&0), None);

    assert_eq!(graph.degree(jakob), 1);
    assert_eq!(graph.degree(edgar), 2);

    assert_eq!(
        &graph
            .edges_of(jakob)
            .into_inner()
            .collect::<HashSet<&EdgeIdx>>(),
        &[&je].into()
    );
    assert_eq!(
        &graph
            .edges_of(edgar)
            .into_inner()
            .collect::<HashSet<&EdgeIdx>>(),
        &[&je, &eb].into()
    );

    assert_eq!(
        &graph
            .neighbors(jakob)
            .into_inner()
            .collect::<HashSet<&NodeIdx>>(),
        &[&edgar].into()
    );
    assert_eq!(
        &graph
            .neighbors(edgar)
            .into_inner()
            .collect::<HashSet<&NodeIdx>>(),
        &[&jakob, &bernhard].into()
    );
}
