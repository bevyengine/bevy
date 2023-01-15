use bevy_graph::graphs::{
    keys::{EdgeIdx, NodeIdx},
    simple::SimpleMapGraph,
    Graph, SimpleGraph,
};
use bevy_utils::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

criterion_main!(benches);

const WARM_UP_TIME: Duration = Duration::from_millis(3000);

criterion_group! {
    name = benches;
    config = Criterion::default().warm_up_time(WARM_UP_TIME);
    targets = nodes_10_000_undirected
}

// TODO: find a better way for fix with multiple iterations over same graph
fn nodes_10_000_undirected(c: &mut Criterion) {
    c.bench_function("nodes_10_000", |b| {
        b.iter(|| {
            let mut graph = SimpleMapGraph::<i32, (), false>::new();

            let mut nodes: Vec<NodeIdx> = Vec::with_capacity(10_000);
            for i in 1..=10_000 {
                nodes.push(graph.new_node(i));
            }

            let mut edges: Vec<EdgeIdx> = Vec::with_capacity(10_000 - 1);
            for i in 1..10_000 {
                edges.push(graph.new_edge(nodes[i - 1], nodes[i], ()).unwrap());
            }

            for edge in &edges {
                black_box(edge.get(&graph));
            }

            for i in 1..10_000 {
                black_box(
                    graph
                        .edge_between(nodes[i - 1], nodes[i])
                        .unwrap()
                        .unwrap()
                        .remove(&mut graph)
                        .unwrap(),
                );
            }
        })
    });
}
