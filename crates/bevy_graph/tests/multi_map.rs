use bevy_graph::{
    graphs::{
        keys::{EdgeIdx, NodeIdx},
        map::MultiMapGraph,
        DirectedGraph, Graph,
    },
    utils::wrapped_indices_iterator::WrappedIndicesIterator,
};
use hashbrown::HashSet;

#[test]
fn undirected() {
    let mut graph = MultiMapGraph::<&str, i32, false>::new();

    assert!(!graph.is_directed());
    assert!(graph.is_multigraph());

    assert_eq!(graph.node_count(), 0);
    let jakob = graph.add_node("Jakob");
    let edgar = graph.add_node("Edgar");
    let bernhard = graph.add_node("Bernhard");
    let no_friends_manny = graph.add_node("No Friends Manny");
    let ego = graph.add_node("Ego");
    assert_eq!(graph.node_count(), 5);

    assert!(graph.contains_node(jakob));
    assert!(graph.contains_node(edgar));
    assert!(graph.contains_node(bernhard));
    assert!(graph.contains_node(no_friends_manny));
    assert!(graph.contains_node(ego));

    assert_eq!(graph.find_node(&"Edgar"), Some(edgar));
    assert_eq!(graph.find_node(&"NoIReallyDon'tExist"), None);

    assert_eq!(
        graph.node_indices().collect::<HashSet<NodeIdx>>(),
        [jakob, edgar, bernhard, no_friends_manny, ego].into()
    );

    assert_eq!(graph.edge_count(), 0);
    let je = graph.add_edge(jakob, edgar, 12);
    let je2 = graph.add_edge(jakob, edgar, 2);
    let eb = graph.add_edge(edgar, bernhard, 7);
    let ego_self = graph.add_edge(ego, ego, 3);
    assert_eq!(graph.edge_count(), 4);

    assert!(graph.contains_edge_between(jakob, edgar));
    assert!(graph.contains_edge_between(edgar, jakob));
    assert!(!graph.contains_edge_between(jakob, bernhard));
    assert!(graph.contains_edge_between(ego, ego));

    assert_eq!(graph.find_edge(&12), Some(je));
    assert_eq!(graph.find_edge(&2), Some(je2));
    assert_eq!(graph.find_edge(&0), None);

    assert_eq!(
        graph.edge_indices().collect::<HashSet<EdgeIdx>>(),
        [je, je2, eb, ego_self].into()
    );

    //assert_eq!(graph.degree(jakob), 2);
    //assert_eq!(graph.degree(edgar), 3);

    assert_eq!(
        graph
            .edges_of(jakob)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [je, je2].into()
    );
    assert_eq!(
        graph
            .edges_of(edgar)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [je, je2, eb].into()
    );

    assert_eq!(
        graph
            .neighbors(jakob)
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [edgar].into()
    );
    assert_eq!(
        graph
            .neighbors(edgar)
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [jakob, bernhard].into()
    );

    assert_eq!(
        graph
            .isolated()
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [no_friends_manny].into()
    );

    assert_eq!(
        graph
            .edges_of(ego)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [ego_self].into()
    );

    assert!(graph.contains_edge_between(edgar, bernhard));
    graph.remove_edge(eb);
    assert_eq!(graph.edge_count(), 3);

    assert!(!graph.contains_edge_between(edgar, bernhard));
    assert!(graph.contains_edge_between(jakob, edgar));
    assert!(graph.contains_edge_between(edgar, jakob));
    assert!(!graph.contains_edge_between(jakob, bernhard));
    assert!(graph.contains_edge_between(ego, ego));

    graph.remove_node(edgar);
    assert_eq!(graph.node_count(), 4);

    assert!(!graph.contains_node(edgar));
    assert!(graph.contains_node(jakob));
    assert!(graph.contains_node(bernhard));
    assert!(graph.contains_node(no_friends_manny));
    assert!(graph.contains_node(ego));

    assert_eq!(graph.edge_count(), 1);
}

/*#[test]
fn directed() {
    let mut graph = MultiMapGraph::<&str, i32, true>::new();

    assert!(graph.is_directed());
    assert!(graph.is_multigraph());

    assert_eq!(graph.node_count(), 0);
    let jakob = graph.add_node("Jakob");
    let edgar = graph.add_node("Edgar");
    let bernhard = graph.add_node("Bernhard");
    let no_friends_manny = graph.add_node("No Friends Manny");
    assert_eq!(graph.node_count(), 4);

    assert!(graph.contains_node(jakob));
    assert!(graph.contains_node(edgar));
    assert!(graph.contains_node(bernhard));
    assert!(graph.contains_node(no_friends_manny));

    assert_eq!(graph.find_node(&"Edgar"), Some(edgar));
    assert_eq!(graph.find_node(&"NoIReallyDon'tExist"), None);

    assert_eq!(
        graph.node_indices().collect::<HashSet<NodeIdx>>(),
        [jakob, edgar, bernhard, no_friends_manny].into()
    );

    assert_eq!(graph.edge_count(), 0);
    let je = graph.add_edge(jakob, edgar, 12);
    let eb = graph.add_edge(edgar, bernhard, 7);
    assert_eq!(graph.edge_count(), 2);

    assert!(graph.contains_edge_between(jakob, edgar));
    assert!(!graph.contains_edge_between(edgar, jakob));
    assert!(!graph.contains_edge_between(jakob, bernhard));

    assert_eq!(graph.find_edge(&12), Some(je));
    assert_eq!(graph.find_edge(&0), None);

    assert_eq!(
        graph.edge_indices().collect::<HashSet<EdgeIdx>>(),
        [je, eb].into()
    );

    assert_eq!(graph.degree(jakob), 1);
    assert_eq!(graph.degree(edgar), 2);
    assert_eq!(graph.out_degree(edgar), 1);
    assert_eq!(graph.in_degree(edgar), 1);

    assert_eq!(
        graph
            .edges_of(jakob)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [je].into()
    );
    assert_eq!(
        graph
            .edges_of(edgar)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [je, eb].into()
    );
    assert_eq!(
        graph
            .incoming_edges_of(edgar)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [je].into()
    );
    assert_eq!(
        graph
            .outgoing_edges_of(edgar)
            .into_indices()
            .collect::<HashSet<EdgeIdx>>(),
        [eb].into()
    );

    assert_eq!(
        graph
            .neighbors(jakob)
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [edgar].into()
    );
    assert_eq!(
        graph
            .neighbors(edgar)
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [jakob, bernhard].into()
    );
    assert_eq!(
        graph
            .in_neighbors(edgar)
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [jakob].into()
    );
    assert_eq!(
        graph
            .out_neighbors(edgar)
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [bernhard].into()
    );

    assert_eq!(
        graph
            .isolated()
            .into_indices()
            .collect::<HashSet<NodeIdx>>(),
        [no_friends_manny].into()
    );

    assert_eq!(
        graph.sources().into_indices().collect::<HashSet<NodeIdx>>(),
        [jakob, no_friends_manny].into()
    );
    assert_eq!(
        graph.sinks().into_indices().collect::<HashSet<NodeIdx>>(),
        [bernhard, no_friends_manny].into()
    );

    graph.reverse();

    assert_eq!(
        graph.sinks().into_indices().collect::<HashSet<NodeIdx>>(),
        [jakob, no_friends_manny].into()
    );
    assert_eq!(
        graph.sources().into_indices().collect::<HashSet<NodeIdx>>(),
        [bernhard, no_friends_manny].into()
    );

    assert!(!graph.contains_edge_between(jakob, edgar));
    assert!(graph.contains_edge_between(edgar, jakob));

    graph.reverse_edge(je);

    assert!(graph.contains_edge_between(jakob, edgar));
    assert!(!graph.contains_edge_between(edgar, jakob));

    graph.reverse_edge(eb); // just for more readable tests - rereverse it

    assert!(graph.contains_edge_between(edgar, bernhard));
    graph.remove_edge(eb);
    assert_eq!(graph.edge_count(), 1);

    assert!(!graph.contains_edge_between(edgar, bernhard));
    assert!(graph.contains_edge_between(jakob, edgar));
    assert!(!graph.contains_edge_between(jakob, bernhard));

    graph.remove_node(edgar);
    assert_eq!(graph.node_count(), 3);

    assert!(!graph.contains_node(edgar));
    assert!(graph.contains_node(jakob));
    assert!(graph.contains_node(bernhard));
    assert!(graph.contains_node(no_friends_manny));

    assert_eq!(graph.edge_count(), 0);
}
*/
